# Generating Mutations

This is a mutation generator for Solidity.
It takes as input a Solidity source file (or a configuration file as you can see below)
and produces a set of uniquely mutated Solidity source files which are output in the `out/` directory by default.
In addition to the mutated source files, Gambit also produces a JSON report of the mutants produced, which can
be found in `out/results.json`. More details on Gambit and integration with the Certora prover can be found [here](https://docs.certora.com/en/latest/docs/gambit/index.html).

## Installing Gambit
- Gambit is implemented in Rust, which you can download [here](https://www.rust-lang.org/tools/install).
- To run Gambit, do the following:
    - `git clone git@github.com:Certora/gambit.git`
    - Install by running `cargo install --path .` from the `gambit/` directory after you clone the repository. This will add the Gambit binary to your `.cargo` directory.
   - You will need OS specific binaries for various versions of Solidity.
  The version of the binary will depend on your Solidity project.
  You can download them
  [here](https://github.com/ethereum/solc-bin). Make sure you add them to your `PATH`.

## Usage
- If you installed Gambit using `cargo install --path .` described above,
  you can learn how to use Gambit by running `gambit mutate -h`.
- If you went for a local build, you can run `cargo gambit-help` for help.
- You can print log messages by setting the environment variable
  `RUST_LOG` (e.g., `RUST_LOG=info cargo gambit ...`).
  This will show colored diffs of the mutants on your standard output.

`cargo gambit-help` lists all the command line arguments that Gambit accepts. The arguments include
`num-mutants (default 5)`, which lets you control the number of mutants you want to generate, 
the `seed (default 0)` that controls the randomization of the generated mutants,
and `outdir (default out)`, which lets you choose where you want to output the mutant files.

These flags are explained in more detail in the following section.

### Examples of How to Run Gambit
You can run Gambit on a single Solidity file with various additional arguments.
Gambit also accepts a configuration file as input where you can
specify which files you want to mutate and using which mutations.
You can also control which functions and contracts you want to mutate.
**Configuration files are the recommended way to use Gambit.**

#### Running Gambit on a Single Solidity File
We recommend this approach only when you have a simple project with few files
and no complex dependencies or mutation requirements.

- `cargo gambit benchmarks/RequireMutation/RequireExample.sol` is an example
  of how to run with a single Solidity file.
- For projects that have complex dependencies and imports, you will likely need to:
    * To specify the Solidity [base path][basepath], pass the `--base-path` argument.  For example
      ```bash
      cargo gambit path/to/file.sol --solc-basepath base/path/dir/.
      ```
    * To indicate where Solidity should find libraries, you provide an [import remapping][remapping] to `solc` using the `--solc-remapping` argument.  For example:
      ```bash
      cargo gambit path/to/file.sol \
        --solc-remapping @openzepplin=node_modules/@openzeppelin \
        --solc-remapping ...
      ```
    * To include additional allowed paths,
      you provide Solidity's [allowed paths][allowed] to `solc` using the `--allow-paths` argument.
      For example:
      ```bash
      cargo gambit path/to/file.sol --solc-allowpaths @openzepplin=... --solc-allowpaths ...
      ```

[remapping]: https://docs.soliditylang.org/en/v0.8.17/path-resolution.html#import-remapping
[basepath]: https://docs.soliditylang.org/en/v0.8.17/path-resolution.html#base-path-and-include-paths
[allowed]: https://docs.soliditylang.org/en/v0.8.17/path-resolution.html#allowed-paths

#### Running Gambit Through a Configuration File
This is the recommended way to run Gambit.
This approach allows you to control and localize
mutation generation and is easier
to use than passing many command line flags.

To run gambit with a configuration file, simply pass the name of the `json` file:
```bash
cargo gambit-cfg benchmarks/config-jsons/test1.json
```

The configuration file is a [json][json-spec] file containing the command line
arguments for `gambit` and additional configuration options.
For example, the following configuration is equivalent
to `gambit benchmarks/10Power/TenPower.sol --solc-remapping @openzepplin=node_modules/@openzeppelin`:

```json
{
    "filename": "benchmarks/10Power/TenPower.sol",
    "remappings": [
        "@openzeppelin=node_modules/@openzeppelin"
    ]
}
```

In addition to specifying the command line arguments, you can list the
specific {ref}`types of mutations <mutation-types>` that you want to apply, the
specific functions you wish to mutate, and more.  See {ref}`gambit-config` for
more details, and [the `benchmark/config-jsons` directory][config-examples] for
examples.

[json-spec]: https://json.org/
[config-examples]: https://github.com/Certora/gambit/blob/master/benchmarks/config-jsons/
[test6]: https://github.com/Certora/gambit/blob/master/benchmarks/config-jsons/test6.json


#### Configuring the Set of Mutations, Functions, and Contracts
If you are using Gambit through a configuration file,
you can localize the mutations to some
functions and contracts.
You can also choose which mutations you want (see {ref}`mutation-types` for the list of possible mutations).
Here is an example that shows how to configure these options.
```
[
    {
        "filename": "Foo.sol",
        "contract": "C",
        "functions": ["bar", "baz"],
        "solc": "solc5.12"
    },
    {
        "filename": "Blip.sol",
        "contract": "D",
        "functions": ["bang"],
        "solc": "solc5.12"
        "mutations": [
          "binary-op-mutation",
          "swap-arguments-operator-mutation"
        ]
    }
]
```

This configuration file will perform all mutations on `Foo.sol`'s
functions `bar` and `baz` in the contract `C`, and
only `binary-op-mutation` and `swap-arguments-operator-mutation` mutations
on the function `bang` in the contract `D`.
Both will compile using the Solidity compiler version `solc5.12`.

### Output of Gambit
Gambit produces a set of uniquely mutated Solidity source
files which are, by default, dumped in
the `out/` directory.
Each mutant file has a comment that describes the exact mutation that was done.
For example, one of the mutant files for
`benchmarks/10Power/TenPower.sol` that Gambit generated contains:
```
/// SwapArgumentsOperatorMutation of: uint256 res = a ** decimals;
uint256 res = decimals ** a;
```

Also included in the `out/` directory is a JSON summary of all mutants produced, `out/results.json`.
The results include the filename and unique string ID of each mutant, along with
a brief description and the `diff` between the mutant and the original file.

### Demo
Here is a demo of Gambit generating mutants for [AaveTokenV3.sol](https://github.com/Certora/aave-token-v3/blob/main/src/AaveTokenV3.sol).
You can clone the Aave repo and then run Gambit with a config file like:

```
{
    "filename": "PATH/TO/aave-token-v3/src/AaveTokenV3.sol",
    "solc-basepath": "PATH/TO/aave-token-v3/.",
    "contract": "AaveTokenV3",
}
```

<img src="doc/gambit-animation.jif" height="450">

## Mutation Types
At the moment, Gambit implements the following types of mutations, listed below.
Many of these mutations may lead to invalid mutants
that do not compile.
At the moment, Gambit simply compiles the mutants and only keeps valid ones &mdash;
we are working on using additional type information to reduce the generation of
invalid mutants by constructions.

Gambit does not apply any mutations to libraries unless they are
explicitly passed as arguments.

What follows is a list of supported mutation types which may be specified in the configuration file.
For more details on each mutation type, refer to the [full documentation](https://docs.certora.com/en/latest/docs/gambit/gambit.html#mutation-types).

```
binary-op-mutation
unary-operator-mutation
require-mutation
assignment-mutation
delete-expression-mutation
function-call-mutation
if-statement-mutation
swap-arguments-function-mutation
swap-arguments-operator-mutation
swap-lines-mutation
elim-delegate-mutation
```

### Contact
If you have ideas for interesting mutations or other features,
we encourage you to make a PR or [email](mailto:chandra@certora.com) us.

### Credits
We thank
[Oliver Flatt](https://www.oflatt.com/) and
[Vishal Canumalla](https://homes.cs.washington.edu/~vishalc/)
for their excellent contributions to an earlier prototype of Gambit.