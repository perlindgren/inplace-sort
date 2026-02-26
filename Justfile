alias b := build
alias w := watch
alias d := diagrams
alias o := open
alias ed := edit-diagram

build:
    nix develop -c make all

watch:
    nix develop -c make watch

diagrams:
    nix develop -c make diagrams

open:
    okular build/ecrts.pdf >/dev/null &

edit-diagram FILE:
    nix develop -c drawio {{FILE}} &>/dev/null &

clean:
    make clean

