use std::path::{Path, PathBuf};
use clap::{App};
use rusoto_core::Region;
use rusoto_iam::{Iam,IamClient,CreateAccessKeyRequest,DeleteAccessKeyRequest};
use tokio;
use configparser::ini::Ini;
use std::env;
use std::ffi::OsString;

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

    let credential_path = expand_tilde(env::var_os("AWS_SHARED_CREDENTIALS_FILE").unwrap_or(OsString::from("~/.aws/credentials")).into_string().unwrap()).unwrap();
    let profile = env::var_os("AWS_PROFILE").unwrap_or(OsString::from("default")).into_string().unwrap();
    // Create client with old key
    let region = Region::UsEast1;
    let client = IamClient::new(region);
    // Load profile as ini and get access_key
    let mut config = Ini::new();
    // Change default section name. This allows writing to a "default" section with explicit header.
    // Otherwise keys are written outside any section header.
    config.set_default_section("another_default");
    match config.load(credential_path.to_str().unwrap()) {
        Ok(_) => (),
        Err(e) => panic!("Unable to load credential file. Error: {:?}", e)
    }
    let old_key = config.get(&profile, "aws_access_key_id").unwrap();
    // Create new key
    println!("Creating new key using {}", old_key);
    let create_access_key_req : CreateAccessKeyRequest = Default::default();
    let new_access_key = client.create_access_key(create_access_key_req).await.unwrap();
    // update config
    config.set(&profile,"aws_access_key_id",Some(new_access_key.access_key.access_key_id));
    config.set(&profile,"aws_secret_access_key",Some(new_access_key.access_key.secret_access_key));
    match config.write(credential_path.to_str().unwrap()) {
        Ok(_) => (),
        Err(e) => panic!("Unable to write credential file. Error: {:?}", e)
    };
    println!("Saving {} in {} profile", config.get(&profile, "aws_access_key_id").unwrap(), profile);
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
        Err(e) => println!("Error deleting key: {:?}", e)
    }

}
