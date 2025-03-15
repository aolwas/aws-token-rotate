use aws_config::meta::region::RegionProviderChain;
use aws_sdk_iam::Client as iamClient;
use clap::Command;
use configparser::ini::Ini;
use std::env;
use std::path::{Path, PathBuf};
use tokio;

fn expand_tilde<P: AsRef<Path>>(path_user_input: P) -> Option<PathBuf> {
    let p = path_user_input.as_ref();
    if !p.starts_with("~") {
        return Some(p.to_path_buf());
    }
    if p == Path::new("~") {
        return dirs::home_dir();
    }
    dirs::home_dir().map(|mut h| {
        if h == Path::new("/") {
            // Corner case: `h` root directory;
            // don't prepend extra `/`, just drop the tilde.
            p.strip_prefix("~").unwrap().to_path_buf()
        } else {
            h.push(p.strip_prefix("~/").unwrap());
            h
        }
    })
}

#[tokio::main]
async fn main() {
    let _matches = Command::new("AWS token rotate")
        .version("1.0")
        .about("Simple tool to rotate AWS token: create and save new credentials and drop old ones.\n\nUse AWS_SHARED_CREDENTIALS_FILE or AWS_PROFILE envvars if needed")
        .get_matches();

    let credential_path = expand_tilde(
        env::var("AWS_SHARED_CREDENTIALS_FILE").unwrap_or(String::from("~/.aws/credentials")),
    )
    .expect("Fail to get credential file path");
    let profile = env::var("AWS_PROFILE").unwrap_or(String::from("default"));
    // Create client with old key
    let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
    let config = aws_config::from_env()
        .profile_name(&profile)
        .region(region_provider)
        .load()
        .await;
    let client = iamClient::new(&config);

    // Load profile as ini and get access_key
    let mut config = Ini::new();
    // Change default section name. This allows writing to a "default" section with explicit header.
    // Otherwise keys are written outside any section header.
    config.set_default_section("another_default");
    config
        .load(
            credential_path
                .to_str()
                .expect("Fail to convert path to str"),
        )
        .expect("Unable to load credential file");
    let old_key = config
        .get(&profile, "aws_access_key_id")
        .expect("Fail to get aws_access_key_id from file for given profile");
    // Create new key
    println!("Creating new key using {}", old_key);
    let new_access_key = client
        .create_access_key()
        .send()
        .await
        .expect("Fail to get new credentials");
    // update config
    match new_access_key.access_key {
        Some(keys) => {
            config.set(&profile, "aws_access_key_id", Some(keys.access_key_id));

            config.set(
                &profile,
                "aws_secret_access_key",
                Some(keys.secret_access_key),
            );
            config
                .write(
                    credential_path
                        .to_str()
                        .expect("Fail to convert path to str"),
                )
                .expect("Unable to write credential file");
            println!(
                "Saving {} in {} profile",
                config
                    .get(&profile, "aws_access_key_id")
                    .expect("Fail to get aws_access_key_id from config for given profile"),
                profile
            );
        }
        None => {
            panic!("Empty new credentials, aborting");
        }
    }

    // Recreate client with to use new credentials
    let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
    let config = aws_config::from_env()
        .profile_name(&profile)
        .region(region_provider)
        .load()
        .await;
    let client = iamClient::new(&config);
    // delete old key
    println!("Deleting {}", old_key);
    match client
        .delete_access_key()
        .set_access_key_id(Some(old_key))
        .send()
        .await
    {
        Ok(_) => println!("Done"),
        Err(e) => panic!("Error deleting key: {:?}", e),
    }
}
