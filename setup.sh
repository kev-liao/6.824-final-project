#!/bin/bash

# Setup on Ubuntu 20.04

# Install dependencies
apt-get update && sudo apt-get upgrade
apt install rustc pkg-config g++ m4 zlib1g-dev make p7zip libflint-dev libssl-dev

# Install Flint
wget http://www.flintlib.org/flint-2.7.1.tar.gz
tar -xf flint-2.7.1.tar.gz
cd flint-2.7.1
./configure
make
make check
make install

apt remove libflint-2.5.2

# Increase open files limit
str="\
*         hard    nofile      500000\n\
*         soft    nofile      500000\n\
root      hard    nofile      500000\n\
root      soft    nofile      500000"

printf "$str" >> /etc/security/limits.conf
