
### Project commands

#### Init local database with docker
`./scripts/init_db.sh`

#### Init local database without init docker container
`SKIP_DOCKER=true ./scripts/init_db.sh`

#### Run project
`cargo run`

#### Execute automated test
`cargo test`

#### Build release to prod
`cargo build --release`

### Digital Ocean commands
#### Init doctl project
`doctl auth init`

#### Create an app with a spec file
`doctl apps create --spec spec.yaml`

#### List all apps
`doctl apps list`

#### Update an app with a new spec file
`doctl apps list --format ID`

`doctl apps update $APP_ID --spec spec.yaml`

### SQLX commands
#### Install sqlx-cli
`cargo install sqlx-cli --no-default-features --features postgres`

#### Create new database migration
`sqlx migrate add migration-name`

#### Run migration
`sqlx migrate run`