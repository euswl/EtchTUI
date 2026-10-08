EtchTUI is a ISO writing TUI for creating bootable USB's with an ISO file. (Balenaetcher in a TUI)

--DEPENDICIES--
Cargo 
Git
Rust

once repo is cloned run: 
cd ~/EtchTUI

Then run: 
cargo install --path ~/EtchTUI/src/main.rs


Once it has installed the path needs to be put into your shells config file 

Bash: 
For Bash you must open your ~/.bashrc file in text editor and add:
export PATH="$HOME/.cargo/bin:$PATH" 


zsh:
For ZSH you must open ~/.zshrc in text editor and add: 
export PATH="$HOME/.cargo/bin:$PATH"


Fish:
For fish, no config file is edited, simply run:

fish_add_path "$HOME/.cargo/bin"
-- Fish saves this for future sessions.

Once the path has been added to your shell of choice please run: 
etchtui 

Thank you for trying etchtui
