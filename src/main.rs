use aws_config::meta::region::RegionProviderChain;
use aws_sdk_iam::Client;
use clap::Command;
use configparser::ini::Ini;
use shellexpand;
use std::env;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _matches = Command::new("AWS token rotate")
            .version("2.0.0")
            .about("Simple tool to rotate AWS token: create and save new credentials and drop old ones.\n\nUse AWS_SHARED_CREDENTIALS_FILE or AWS_PROFILE envvars if needed")
            .get_matches();
    let credentials_path = shellexpand::tilde(
        env::var("AWS_SHARED_CREDENTIALS_FILE")
            .unwrap_or(String::from("~/.aws/credentials"))
            .as_str(),
    )
    .to_string();
    let profile_name = env::var("AWS_PROFILE").unwrap_or(String::from("default"));

    // Step 1: Read the current credentials file
    let mut creds_ini = Ini::new();
    // Change default section name. This allows writing to a "default" section with explicit header.
    // Otherwise keys are written outside any section header.
    creds_ini.set_default_section("another_default");
    creds_ini.load(credentials_path.as_str())?;

    let old_access_key_id = creds_ini
        .get(&profile_name, "aws_access_key_id")
        .expect("Unable to load current access key id");

    // Step 2: Create new IAM access keys for the user
    let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
    let config = aws_config::from_env()
        .profile_name(&profile_name)
        .region(region_provider)
        .load()
        .await;
    let iam_client = Client::new(&config);

    println!("Creating new key using {}", old_access_key_id);
    match iam_client.create_access_key().send().await {
        Ok(response) => {
            let new_access_key = response
                .access_key
                .expect("Fail to get access key from response");
            let access_key_id = new_access_key.access_key_id;
            let secret_access_key = new_access_key.secret_access_key;
            println!("Saving {} in {} profile", access_key_id, profile_name);
            creds_ini.set(&profile_name, "aws_access_key_id", Some(access_key_id));
            creds_ini.set(
                &profile_name,
                "aws_secret_access_key",
                Some(secret_access_key),
            );

            creds_ini
                .write(credentials_path.as_str())
                .expect("Fail to persist new credentials");

            // Step 2bis: delete old key.
            // NOTE: try to recreate client with new credentials but delete request always fails
            println!("Deleting {}", old_access_key_id);
            match iam_client
                .delete_access_key()
                .access_key_id(old_access_key_id)
                .send()
                .await
            {
                Ok(_) => println!("Done."),
                Err(err) => {
                    eprintln!(
                        "Unexpected error deleting old access key: {}",
                        err.into_service_error()
                    );
                }
            }
        }
        Err(err) => {
            eprintln!("Failed to create access keys: {}", err);
        }
    }

    Ok(())
}
