# Init

This is an init binary, much like AWS' [init.c](https://github.com/aws/aws-nitro-enclaves-sdk-bootstrap/blob/746ec5d2713e539b94e651601b5c24ec1247c955/init/init.c)
except
- it is written in Go
- it is compatible with a missing nsm binary and a linux kernel 6.8+ (which [includes the nsm module](https://fosslinux.community/forum/news-ideas/linux-6-8-released-a-milestone-with-intel-gpu-aws-nitro-support-and-a-peek-into-6-9s-future/))


This init is still compatible with kernels older than 6.8 by loading nsm if it does find one.