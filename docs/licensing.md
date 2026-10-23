# License files and packaging

The root `LICENSE-MIT` and `LICENSE-APACHE` files contain the project's license
texts. Each publishable workspace crate links to both files using relative Git
symlinks. Keep these links as symlinks (Git mode `120000`), rather than text files
containing `../LICENSE-MIT` or `../LICENSE-APACHE`.

[Cargo flattens real symlinks](https://doc.rust-lang.org/cargo/commands/cargo-package.html)
when creating `.crate` archives, so registry users receive complete, ordinary
license files. Check the links and packaged license contents before publishing:

```sh
python3 scripts/check-licenses.py
cargo package --workspace --no-verify
python3 scripts/check-licenses.py --archives
```

`--no-verify` skips compiling the packaged crates; it does not replace the normal
build and test checks or the release workflow's `cargo package` verification.
For an individual crate, pass `-p <name>` to Cargo and `--package <name>` to the
script. The archive check uses Cargo's configured target directory.

## Windows checkouts

Git can check out symlinks as ordinary files containing only their target paths
when `core.symlinks=false`. Cargo cannot turn those files into the license texts.
The integrity check rejects such checkouts before packaging.

Before cloning for packaging on Windows, enable symbolic-link support (for
example, Windows Developer Mode) and use:

```sh
git clone -c core.symlinks=true https://github.com/poem-web/poem.git
```

Setting `core.symlinks` after cloning does not convert existing pointer files.
Use a fresh symlink-enabled checkout, or package from Linux/WSL. The release
workflow packages on Linux and validates both the checkout and the archives.
