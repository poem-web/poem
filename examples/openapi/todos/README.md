# TODOs Example

## Setup

1. Install sqlx-cli

```bash
cargo install sqlx-cli --no-default-features --features sqlite
```

2. Declare the database URL

```bash
export DATABASE_URL="sqlite:todos.db"
```

3. Create the database

```bash
sqlx db create
```

4. Run sql migrations

```bash
sqlx migrate run
```

5. Start the server

```bash
cargo run
```
