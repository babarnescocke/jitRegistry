# jitRegistry: A container registry-like service

This program aims to create containers on a just-in-time basis for pushing to users.

# Why registry-like?

The registry specifications, Docker and OCI, require things that this program simply cannot have, such as being able to return the hash of all of the manifests, by definition a container not yet built, doesn't have a known hash. Perhaps, we will get around to making a real solution, but until then, registry-like.


# Program Dependencies:
 	* linux kernel > 3.14
 	* buildah
 		- probably can't be containerized because of buildah using namespaces for mounting pseudo-filesystems.