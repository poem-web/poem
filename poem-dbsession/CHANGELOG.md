# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

# [0.1.4] 2021-12-05

- No longer automatically clean up expired sessions in the database.
- Expose `cleanup` method.

# [0.1.3] 2021-12-03

- Change the return type of `MysqlSessionStorage::new`/`PgSessionStorage::try_new`/`SqliteSessionStorage::try_new` to `sqlx::Result`.

# [0.1.1] 2021-12-03

- Rename `MysqlSessionStorage::new` to `MysqlSessionStorage::try_new`.
- Rename `PgSessionStorage::new` to `MysqlSessionStorage::try_new`.
- Rename `SqliteSessionStorage::new` to `MysqlSessionStorage::try_new`.
