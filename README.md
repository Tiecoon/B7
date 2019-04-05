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

## Documentation

Currently there is no hosted documentation but you can get local docs with

```
cargo docs --open
```

## Contribute

To get involved in the project, read the [Contribution Guidelines](./CONTRIBUTION.md)
