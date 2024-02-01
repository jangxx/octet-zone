# OCTET ZONE

> A dynamic DNS server that brings octet notation to IPv6.

Read more about this project over at https://octet.zone.

## Running the server

Running the server should be fairly straight forward, so I'm just going to leave the help output here:

```
Usage: octet-zone [OPTIONS]

Options:
  -d, --domain <DOMAIN>       Domain name [env: OCTETZONE_DOMAIN=] [default: octet.zone]
  -u, --udp <UDP>             UDP socket to listen on [env: OCTETZONE_LISTEN=] [default: 0.0.0.0:1053]
  -4, --root-v4 <ROOT_IPV4>   IPV4 address to resolve to for the root domain name
  -6, --root-v6 <ROOT_IPV6>   IPV6 address to resolve to for the root domain name
      --txt <ADDITIONAL_TXT>  Additional TXT records to resolve (format: name=value)
  -h, --help                  Print help
  -V, --version               Print version
```

If you want to use this as a "real" DNS server, you definitely want to change the `-u` option to run on port 53 and also the `-d` option to run on a domain that you own, instead of "octet.zone".

The `-4`, `-6` and `--txt` entries can occur more than once in order to resolve the root to multiple addresses or to add TXT records whereever you want.
The format for the TXT records is `name=value`, so setting it to (for example) `bla.octet.zone=asdf` would lead to this response:

```
$ dig @127.0.0.1 -p 1053 bla.octet.zone +noall +answer -t TXT
bla.octet.zone.		3600	IN	TXT	"asdf"
```