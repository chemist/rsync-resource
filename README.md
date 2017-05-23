##Rsyncd settings

Some thing like this
```
uid = nobody
gid = nobody
use chroot = no
max connections = 100
syslog facility = local5
pid file = /var/run/rsyncd/rsyncd.pid
lock file = /var/run/rsyncd/rsync.lock
read only = no
write only = no
hosts allow = 192.168.0.0/24
hosts deny = *

[current]
  path = /var/repo/7
  comment = rpm repository for centos7
#
# mkdir -p /var/repo/7/prod
#

[bin]
  path = /opt/rsync/bin
  comment = temporary store 
#
# mkdir -p /opt/rsync/bin
#
[static]
  path = /opt/rsync/static
  comment = input point for static files
#
# mkdir -p /opt/rsync/static
#
```
###Example

``` yaml
resource_types: ###################################################################################################
- name: rsync-resource
  type: docker-image
  source:
    repository: chemist/rsync-resource
    tag: latest

resources: #######################################################################################################
- name: rpm-7
  type: rsync-resource
  source:
    server: rsync.server.io
    base_dir: current
    resource_type: w
    static_identificator: prod

# write only resource
# created artifacts will be saved to /var/repo/7/prod
#  - put: rpm-7
#    params:
#      sync_dir: rpm-7


- name: bin
  type: rsync-resource
  source:
    server: rsync.server.io
    base_dir: bin
    resource_type: rw

# job example
# read - write changer, this resource can be used for moving artifacts between jobs
# folders will be created for every run /opt/rsync/bin/uniq-prefix-$(date)
#  - put: bin
#    params:
#      sync_dir: bin
#      identificator: uniq-prefix # this prefix will be used for folders
#  - get: bin
#    trigger: true
#    passed: [build-dsp]


- name: static
  type: rsync-resource
  source:
    server: rsync.server.io
    base_dir: static
    static_identificator: input-prod
    resource_type: r 

# job example
# read only store, if your job needs static file, it is what you need.
#  - get: static
#    trigger: false
# it checks new files in folder /opt/rsync/static/input-prod

```

