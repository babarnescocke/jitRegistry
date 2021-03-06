# jitRegistry: A container registry-like service

This program aims to create containers on a just-in-time basis for serving to users.

# What jitRegistry does

We have some directory structure with subdirectories of either [buildah](https://buildah.io/) [scripts](https://www.redhat.com/sysadmin/getting-started-buildah), or Dockerfiles. As an example:

testRegistry
├── foo
│   ├── buildahScript.sh
│   └── buildFiles
│       ├── arbitrary.bin
│       ├── bar.conf
│       └── entrypoint.sh
└── nginxAlpine
    └── Dockerfile
```
jitregistry --directory-path="path/to/testRegistry"
podman run localhost:7999/images/foo
```
jitRegistry fires buildah to build foo/buildahScript.sh and serves it.

# Why registry-like?

The registry specifications, Docker and OCI, require things that this program simply cannot have, such as being able to return the hash of all of the manifests, by definition a container not yet built, doesn't have a known hash. Perhaps, we will get around to making a real solution, but until then, registry-like.


# Program Dependencies
 	* linux kernel > 3.14
 	* buildah
 		- probably can't be containerized because of buildah using namespaces for mounting pseudo-filesystems.