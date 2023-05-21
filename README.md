# k6parser

![parse result example](./docs/example.png)

parse k6 result

```
Usage: k6parser [OPTIONS] <FILE> [OUTPUT_TYPE]

Arguments:
  <FILE>
  [OUTPUT_TYPE]  [possible values: html, json]

Options:
  -o, --output <OUTPUT>
  -h, --help             Print help
  -V, --version          Print version
```


## installing k6parser

```
$ cargo install --git https://github.com/sakti/k6parser
```

## example

```
$ k6 run --out json=katze.gz katze.js
$ k6parser katze.gz
```


## tested on

```
$ k6 version
k6 v0.43.1 ((devel), go1.20.1, darwin/arm64)
```
