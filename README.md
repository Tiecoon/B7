# B7

[![Build Status](https://gitlab.com/tiecoon/B7/badges/master/build.svg)](https://gitlab.com/tiecoon/B7/pipelines)

A WIP project that aims to ease/automatically perform instruction counting by running programs with
various input under multiple harnesses like perf or Dynamorio.

## Installation

Currently only installs on linux systems due to perf not being separated yet

To install run:

```
git submodule init
git submodule update
cargo install --path .
```

## Requirements

* Necessary
	* Dynamorio
		* https://github.com/DynamoRIO/dynamorio/wiki/How-To-Build
		* currently requires both a c and c++ compiler and multilib/32 bit counterparts
* Optional
	* perf
		* linux-perf-x.xx for your kernel version
		* sysctl kernel.perf_event_paranoid < 3

## Testing

To run multiple tests at the same time pass `-- --test-threads=1` to cargo test

## Documentation

Currently there is no hosted documentation but you can get local docs with

```
cargo docs --open
```

## Contribute

To get involved in the project, read the [Contribution Guidelines](./CONTRIBUTION.md)

## Communication

The B7 blog is located in the Rensselaer Center for Open Source Observatory project page here: https://rcos.io/projects/tiecoon/b7/](https://rcos.io/projects/tiecoon/b7/profile

## Conduct

Please abide by the [B7 Code of Conduct](./CodeOfConduct.md) when contributing to the project and interacting with the community.
