# pleme-database

Database utilities library for Pleme platform - connection pooling, transactions, repository pattern

## Installation

```toml
[dependencies]
pleme-database = "0.1"
```

## Usage

```rust
use pleme_database::{DatabasePool, Repository};

let pool = DatabasePool::connect(&database_url).await?;

#[async_trait]
impl Repository for UserRepo {
    type Entity = User;
    async fn find_by_id(&self, id: Uuid) -> Result<User> {
        // SQLx query
    }
}
```

## Feature Flags

| Feature | Description |
|---------|-------------|
| `postgres` | PostgreSQL support via SQLx (default) |
| `cache` | Redis caching layer |
| `errors` | pleme-error integration |
| `full` | All features enabled |

Enable features in your `Cargo.toml`:

```toml
pleme-database = { version = "0.1", features = ["full"] }
```

## Development

This project uses [Nix](https://nixos.org/) for reproducible builds:

```bash
nix develop            # Dev shell with Rust toolchain
nix run .#check-all    # cargo fmt + clippy + test
nix run .#publish      # Publish to crates.io (--dry-run supported)
nix run .#regenerate   # Regenerate Cargo.nix
```

## License

MIT - see [LICENSE](LICENSE) for details.
