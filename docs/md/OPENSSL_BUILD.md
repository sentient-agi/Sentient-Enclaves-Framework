## **OpenSSL static build guide:**

The following is a guide of how to build attestation web server with VRF proofs library with the most recent OpenSSL library, built from sources and statically bound/link it with resulted binary/executable.

```bash
wget https://github.com/openssl/openssl/releases/download/openssl-3.5.0/openssl-3.5.0.tar.gz
tar -xvf openssl-3.5.0.tar.gz 

cd ./openssl-3.5.0
./Configure
sudo mkdir -p /usr/local/ssl
./config --prefix=/usr/local/ssl

make
make test
sudo make install

sudo ln -sf /usr/local/ssl/bin/openssl /usr/bin/openssl
sudo ln -sf /usr/local/ssl/lib64/libssl.so /usr/lib64/libssl.so
sudo ln -sf /usr/local/ssl/lib64/libssl.so /usr/lib64/libssl.so.3
sudo ln -sf /usr/local/ssl/lib64/libcrypto.so /usr/lib64/libcrypto.so
sudo ln -sf /usr/local/ssl/lib64/libcrypto.so /usr/lib64/libcrypto.so.3

openssl version

# Command to build project, include most recent OpenSSL built from sources and statically bound/link it with resulted binary/executable
OPENSSL_STATIC="" OPENSSL_NO_VENDOR="" OPENSSL_DIR="/usr/local/ssl/" OPENSSL_LIB_DIR="/usr/local/ssl/lib64/" OPENSSL_INCLUDE_DIR="/usr/local/ssl/include/" cargo build --all && OPENSSL_STATIC="" OPENSSL_NO_VENDOR="" OPENSSL_DIR="/usr/local/ssl/" OPENSSL_LIB_DIR="/usr/local/ssl/lib64/" OPENSSL_INCLUDE_DIR="/usr/local/ssl/include/" cargo build --release --all
```
