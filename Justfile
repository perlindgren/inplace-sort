alias b := build
alias w := watch
alias d := diagrams
alias o := open

build:
    nix develop -c make all

watch:
    nix develop -c make watch

diagrams:
    nix develop -c make diagrams

open:
    okular build/main.pdf >/dev/null &

clean:
    make clean

