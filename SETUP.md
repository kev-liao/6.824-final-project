## Setup instructions on Ubuntu 20.04

All of these steps can be run using `yes | sudo ./setup.sh`.

1. Update system packages.
```
sudo apt-get update && sudo apt-get upgrade
```

2. Install dependencies.
```
sudo apt-get install rustc pkg-config g++ m4 zlib1g-dev make p7zip libflint-dev
```

3. Install [Flint](http://www.flintlib.org/downloads.html).
```
wget http://www.flintlib.org/flint-2.7.1.tar.gz
tar -xf flint-2.7.1.tar.gz
cd flint-2.7.1
./configure
make
make check
make install
```

4. Run `cargo build`.

5. Increase open files limit.
Open `/etc/security/limits.conf` and append the following:
```
*         hard    nofile      500000
*         soft    nofile      500000
root      hard    nofile      500000
root      soft    nofile      500000
```
