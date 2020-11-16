use std::path::{Path, PathBuf};
use clap::{App};
use rusoto_core::Region;
use rusoto_iam::{Iam,IamClient,CreateAccessKeyRequest,DeleteAccessKeyRequest};
use tokio;
use configparser::ini::Ini;
use std::env;

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
    let _matches = App::new("AWS token rotate")
        .version("1.0")
        .about("Simple tool to rotate AWS token: create and save new credentials and drop old ones.\n\nUse AWS_SHARED_CREDENTIALS_FILE or AWS_PROFILE envvars if needed")
        .get_matches();

    let credential_path = expand_tilde(env::var("AWS_SHARED_CREDENTIALS_FILE").unwrap_or(String::from("~/.aws/credentials"))).expect("Fail to get credential file path");
    let profile = env::var("AWS_PROFILE").unwrap_or(String::from("default"));
    // Create client with old key
    let region = Region::UsEast1;
    let client = IamClient::new(region);
    // Load profile as ini and get access_key
    let mut config = Ini::new();
    // Change default section name. This allows writing to a "default" section with explicit header.
    // Otherwise keys are written outside any section header.
    config.set_default_section("another_default");
    config.load(credential_path.to_str().expect("Fail to convert path to str")).expect("Unable to load credential file");
    let old_key = config.get(&profile, "aws_access_key_id").expect("Fail to get aws_access_key_id from file for given profile");
    // Create new key
    println!("Creating new key using {}", old_key);
    let create_access_key_req : CreateAccessKeyRequest = Default::default();
    let new_access_key = client.create_access_key(create_access_key_req).await.expect("Fail to get new credentials");
    // update config
    config.set(&profile,"aws_access_key_id",Some(new_access_key.access_key.access_key_id));
    config.set(&profile,"aws_secret_access_key",Some(new_access_key.access_key.secret_access_key));
    config.write(credential_path.to_str().expect("Fail to convert path to str")).expect("Unable to write credential file");
    println!("Saving {} in {} profile", config.get(&profile, "aws_access_key_id").expect("Fail to get aws_access_key_id from config for given profile"), profile);
    // Recreate client with to use new credentials
    let region = Region::UsEast1;
    let client = IamClient::new(region);
    // delete old key
    println!("Deleting {}", old_key);
    let delete_access_key_req = DeleteAccessKeyRequest {
        access_key_id: old_key,
        user_name: None,
    };
    match client.delete_access_key(delete_access_key_req).await {
        Ok(_) => println!("Done"),
        Err(e) => panic!("Error deleting key: {:?}", e)
    }
}
