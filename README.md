# Fold - a customizable dynamic linker

## Docker environment

```sh
# Start the container
docker compose run --rm fold

# Build the linker
just build

# Build sqlite
just sqlite-build

# Run
sqlite-build/sqlite3
```
