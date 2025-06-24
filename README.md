# Satisfactory Serve Map

A Rust web service that serves Satisfactory save files for use with the [Satisfactory Calculator Interactive Map](https://satisfactory-calculator.com/en/interactive-map).

## Features

- Serves save files over HTTP for integration with Satisfactory Calculator
- Automatically finds the latest save file for a given save name
- Provides a web interface listing all available saves
- CORS headers configured for Satisfactory Calculator integration
- NixOS module for easy deployment

## NixOS Module

This project includes a NixOS module for easy deployment. Here's how to use it:

### 1. Add to your flake inputs

Add this flake to your `flake.nix` inputs:

```nix
# /etc/nixos/flake.nix
{
  inputs = {
    # ... other inputs
    satisfactory-serve-map.url = "github:kemichal/satisfactory_serve_map";
  };

  outputs = { self, nixpkgs, satisfactory-serve-map, ... }@inputs: {
    nixosConfigurations.your-hostname = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        # ... your other modules
        satisfactory-serve-map.nixosModules.default

        # Configure the service
        {
          services.satisfactory-serve-map = {
            enable = true;
            base_url = "https://sf.example.com";
            save_dir = "/path/to/your/saves";
            port = 7778; # Optional, defaults to 7778
          };
        }
      ];
    };
  };
}
```
### Important Notes

- The `base_url` and `save_dir` options are **required**
- The service user (`satisfactory-serve-map` by default) needs **read access** to the `save_dir`
- The service will automatically create the user and group if they don't exist

## Development

### Getting Started

1. Enter the development shell:
   ```bash
   nix develop
   ```

2. Build the project:
   ```bash
   cargo build
   ```

3. Run the service:
   ```bash
   cargo run
   ```

### Configuration

The service uses a `config.toml` file for configuration. In development, it will look for:
1. `config.dev.toml` (development configuration)
2. `config.toml` (fallback configuration)

Example configuration:
```toml
# Configuration for satisfactory_serve_map

# The base URL where your save files will be accessible
# This URL will be used to construct the links to the Satisfactory Calculator
# Example: "https://sf.example.com"
base_url = "https://your.domain.com"

# Directory containing save files
save_dir = "saves"

# Port to run the server on
port = 7778
```

## API Endpoints

- `GET /map/<name>` - Serves the latest save file for the given save name
- `GET /map` - Serves an HTML page listing all available saves with links to Satisfactory Calculator
