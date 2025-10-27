use anyhow::{Context, Result};
use dialoguer::{Input, Password, Confirm, Select};
use std::path::PathBuf;
use crate::config::{Config, Destination};

pub fn run_setup() -> Result<Config> {
    // Check if config already exists
    if let Some(existing_config) = Config::load()? {
        println!("\n⚠️  Configuration already exists!\n");
        
        let options = vec![
            "Add a new destination",
            "Edit an existing destination", 
            "Delete a destination",
            "Start fresh (delete all and reconfigure)",
            "Cancel",
        ];
        
        let choice = Select::new()
            .with_prompt("What would you like to do?")
            .items(&options)
            .default(0)
            .interact()?;
        
        match choice {
            0 => add_destination(existing_config),
            1 => edit_destination(existing_config),
            2 => delete_destination(existing_config),
            3 => {
                let confirm = Confirm::new()
                    .with_prompt("⚠️  This will delete all your existing settings. Are you sure?")
                    .default(false)
                    .interact()?;
                
                if confirm {
                    setup_fresh()
                } else {
                    println!("\nCancelled.\n");
                    Ok(existing_config)
                }
            }
            _ => {
                println!("\nCancelled.\n");
                Ok(existing_config)
            }
        }
    } else {
        setup_fresh()
    }
}

fn setup_fresh() -> Result<Config> {
    println!("\n🚀 Welcome to arkv! Let's get you set up.\n");

    let ssh_key_path = get_ssh_key_path()?;
    
    println!("\n✓ SSH key configured: {}\n", ssh_key_path);

    let mut destinations = Vec::new();
    
    loop {
        println!("Setting up a remote destination...\n");
        let destination = setup_destination()?;
        destinations.push(destination);

        let add_more = Confirm::new()
            .with_prompt("Add another destination?")
            .default(false)
            .interact()?;

        if !add_more {
            break;
        }
        println!();
    }

    let config = Config {
        ssh_key_path,
        destinations,
    };

    config.save()?;
    
    println!("\n✓ Configuration saved! You're ready to use arkv.\n");
    
    Ok(config)
}

fn add_destination(mut config: Config) -> Result<Config> {
    println!("\n📦 Adding a new destination...\n");
    
    let destination = setup_destination()?;
    config.destinations.push(destination);
    
    config.save()?;
    println!("\n✓ Destination added!\n");
    
    Ok(config)
}

fn edit_destination(mut config: Config) -> Result<Config> {
    if config.destinations.is_empty() {
        println!("\nNo destinations configured.\n");
        return Ok(config);
    }
    
    let names: Vec<String> = config.destinations.iter()
        .map(|d| format!("{} ({})", d.name, d.host))
        .collect();
    
    let selection = Select::new()
        .with_prompt("Select destination to edit")
        .items(&names)
        .default(0)
        .interact()?;
    
    println!("\n📝 Editing {}...\n", config.destinations[selection].name);
    
    let new_dest = setup_destination()?;
    config.destinations[selection] = new_dest;
    
    config.save()?;
    println!("\n✓ Destination updated!\n");
    
    Ok(config)
}

fn delete_destination(mut config: Config) -> Result<Config> {
    if config.destinations.is_empty() {
        println!("\nNo destinations configured.\n");
        return Ok(config);
    }
    
    let names: Vec<String> = config.destinations.iter()
        .map(|d| format!("{} ({})", d.name, d.host))
        .collect();
    
    let selection = Select::new()
        .with_prompt("Select destination to delete")
        .items(&names)
        .default(0)
        .interact()?;
    
    let name = config.destinations[selection].name.clone();
    
    let confirm = Confirm::new()
        .with_prompt(format!("Delete '{}'?", name))
        .default(false)
        .interact()?;
    
    if confirm {
        config.destinations.remove(selection);
        config.save()?;
        println!("\n✓ Destination '{}' deleted!\n", name);
    } else {
        println!("\nCancelled.\n");
    }
    
    Ok(config)
}

fn get_ssh_key_path() -> Result<String> {
    let home = dirs::home_dir().context("Could not find home directory")?;
    let default_key = home.join(".ssh").join("id_ed25519");
    
    let default_str = default_key.to_string_lossy().to_string();
    
    let path: String = Input::new()
        .with_prompt("Path to your SSH private key")
        .default(default_str)
        .interact_text()?;

    let key_path = PathBuf::from(&path);
    if !key_path.exists() {
        anyhow::bail!("SSH key not found at: {}", path);
    }

    Ok(path)
}

fn setup_destination() -> Result<Destination> {
    let name: String = Input::new()
        .with_prompt("Name for this connection")
        .interact_text()?;

    let host: String = Input::new()
        .with_prompt("Server address (e.g., example.com or 192.168.1.1)")
        .interact_text()?;

    let port: u16 = Input::new()
        .with_prompt("SSH port")
        .default(22)
        .interact_text()?;

    let username: String = Input::new()
        .with_prompt("Username")
        .interact_text()?;

    let remote_path: String = Input::new()
        .with_prompt("Remote folder path (e.g., /home/user/uploads)")
        .interact_text()?;

    let use_password = Confirm::new()
        .with_prompt("Use password authentication? (otherwise SSH key will be used)")
        .default(false)
        .interact()?;

    let password = if use_password {
        Some(Password::new()
            .with_prompt("Password")
            .interact()?)
    } else {
        None
    };

    Ok(Destination {
        name,
        host,
        port,
        username,
        remote_path,
        password,
    })
}
