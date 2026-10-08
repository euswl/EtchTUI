EtchTUI is a ISO writing TUI for creating bootable USB's with an ISO file. (Balenaetcher in a TUI)

# DEPENDICIES

1. Cargo
2. Git
3. Rust
  

# INSTALLATION 


once repo is cloned run: 


```sh
cd ~/EtchTUI
```

Then run: 


```sh
cargo install --path ~/EtchTUI/src/main.rs
```

## SHELL CONFIG


Once it has installed the path needs to be put into your shells config file 

| Shell | Action |
|---|---|
| Bash | Open `~/.bashrc` in a text editor and add `export PATH="$HOME/.cargo/bin:$PATH"`. |
| Zsh | Open `~/.zshrc` in a text editor and add `export PATH="$HOME/.cargo/bin:$PATH"`. |
| Fish | Run `fish_add_path "$HOME/.cargo/bin"`. Fish will remember this setting. |



## USAGE

Once the path has been added to your shell of choice please run: 

```sh
etchtui
```

Thank you for trying etchtui
