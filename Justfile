_default:
    @echo "{{arch()}} {{os()}} {{os_family()}} machine"
    @just --choose


test:
    cargo r -- result.gz
