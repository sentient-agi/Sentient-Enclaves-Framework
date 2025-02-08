## Changelog: what's already done and implemented, release notes

### Done:

- **0.1.0** - Pipeline secure local channel protocol, for host to enclave (and vise versa, enclave to host) data transferring,
        and for remote commands execution (with execution results output to logs),
        for managing enclave's environment, providing container-like experience.

- **0.2.0** - Reproducible builds framework for building customized enclave images (EIF),
        and provide experience for "just use your app dockerfile and build enclave".

- **0.3.0** - Forward proxies, transparent forward proxies, port forwarding proxies, for TCP (HTTP/HTTPs, etc.) and UDP (DNS, etc.) protocols.
        Providing networking stack for applications in enclave, to forward traffic from apps, from inside the enclave,
        use high-level networking protocols (network/cloud data storage, web, etc.), supported over VSock.

- **0.4.0** - New updated reproducible builds system for Sentient Enclaves Framework for building customized enclave images (EIF).

- **0.5.0** - Reverse proxies, transparent reverse proxies (including transparent port forwarding to vsock),
        to support request forwarding into enclave's apps, for providing services (mostly web and other network protocols),
        hosted inside enclave (in isolated memory region and isolated environment from host system).

- **0.6.0** - Set of reference applications, built with framework - inference server (will include Dobby model),
        fine-tuning server (includes fine-tuning OML library), X agent (chat bot app).
