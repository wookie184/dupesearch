# dupesearch

A fast and simple command line tool written in rust and python for finding duplicate files.

## Should I use it?

Probably not, this is still a WIP, it's quite messy, I haven't tested most of it, and was my first time using Rust so I probably did some dodgy stuff.

## How do I use it?

1) Install the `dupesearch` package from pip using your preferred command.
2) Run `python -m dupesearch -i` and follow the instructions.

## Contributions

To set up locally:

1) Clone the repository with `git clone https://github.com/wookie184/dupesearch.git`
2) `cd` into the repository with `cd dupesearch`
3) [Create](https://packaging.python.org/guides/installing-using-pip-and-virtual-environments/#creating-a-virtual-environment) and [activate](https://packaging.python.org/guides/installing-using-pip-and-virtual-environments/#activating-a-virtual-environment) a venv, and make sure to point your editor to the executable if necessary.
4) Install the dev requirements with `pip install -r requirements-dev.txt`
5) Install the pre-commit hooks with `pre-commit install`
6) Install the rust crate as a module with `maturin develop --release`
7) Finally, run the project with `python -m dupesearch`

If you'd like to make a contribution, feel free to and i'll try to merge it in. If it's not a tiny change please create an issue first, at least to ensure that i'm still active and able to review the changes.

Any issues suggesting improvements are also welcome, although this was just done as a practice project to get started with rust and packaging python projects, so I may not implement every suggestion.
