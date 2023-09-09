# Gambit: Mutant Generation for Solidity

<!-- SUPPRESS --> 

<!-- NOTE: IF YOU EDIT THIS FILE!!!

  # Keep In Sync With ReadTheDocs


  This documentation appears in two locations: 
  - here (the Gambit repo), and
  - https://github.com/Certora/Documentation/docs/gambit/gambit.md (used to
    generate the ReadTheDocs documentation at
    https://docs.certora.com/en/latest/docs/gambit/gambit.html)

  Unfortunately we cannot simply copy this file the Certora Documentation repo:
  the formats are slightly different (I describe how below). To solve this we
  _generate_ the RTD docs from this file.


  ## Steps to Sync With RTD
  

  1. Run `python scripts/generate_rtd_markdown.py README.md`

     This will create `./gambit.md`

  2. Move this generated `gambit.md` to location `docs/gambit/gambit.md` in the
     Certora Documentation repository (https://github.com/Certora/Documentation)
  
  3. Create a new PR in https://github.com/Certora/Documentation with the new
     Gambit docs
  
  4. Create a new PR in Gambit repo. Note that CI will check that the RTD
     documentation is up to date (see section "Checking RTDs are Up To Date"
     below). If this fails, CI will also fail, and you will be unable to merge
     into `master` until changes to the Gambit README are propagated to the RTD
     docs.
  
  5. Once the PR from (3) is merged and CI is passing in this repository, merge
     the PR from (4) into master.


  ## Checking RTDs are Up To Date

  To check that the RTD Gambit docs are in sync with Gambit's README, run

  ```
  python scripts/check_rtd_docs_upt_to_date.py
  ```

  This will translate the Gambit README to a string, pull the RTD docs from the
  Github Repo, and do a equality check on the two strings.
  
  You can optionally specify a `--branch` argument to choose another branch in
  the Certora Documentation repo (default is `'master'`)


  ## Markdown Format Differences


  1. Internal Links:

     ```markdown
     [Link Title](#link-title) becomes
     ```
     
     ```markdown
     {ref}`link-title`
     ```

     Note that `link-title` needs to target an _anchor_, which we describe in
     item 2.

  2. Anchors:
     
     To link to internal locations the RTD documentation expects an _anchor_:

     ```markdown
     (heading-title)=
     ## Heading Title
     ```

     Then the 

     ```markdown
     {ref}`heading-title`
     ```
     
    internal link will point to the `## Heading Title` section.

    Raw markdown doesn't support this syntax, so we use an HTML comment
    containing the contents "ANCHOR (content-title)=":

    ```markdown
    <!-- ANCHOR (heading-title)=   -- >
    ```

    Note that I intentionally added a space to the close comment tag because
    that would terminate this entire comment :)
  
  3. Notes:

     RTD uses admonition-style notes:

     ```{note}
     Some note goes here
     ```

     We don't have access to this here, so I've implemented a simple system,
     where all notes begin with a line containing:

     ```markdown
     _**Note:**
     ```

     and end with a line ending with `_`:

     ```markdown
     and this is the last line of my note._
     ```

     So, a full note would look like:

     ```markdown
     _**Note:**
     This is a note. The opening tag is on its own line, and the closing italic
     is at the end of the final line. This is to make parsing easy, and to keep
     diffs minimal!_
     ```
     
-->

<!-- END SUPPRESS -->

<!-- EMIT:
<\!--
  WARNING: AUTO_GENERATED DOCUMENTATION

  The following documentation is automatlically generated from the Gambit
  README.md located at https://github.com/Certora/Gambit/README.md. Please view
  this document for instructions on producing this file.
--\>
-->

Gambit is a state-of-the-art mutation system for Solidity.
By applying predefined syntax transformations called _mutation operators_ (for
  example, convert `a + b` to `a - b`) to a Solidity program's source code, Gambit
  generates variants of the program called _mutants_.
Mutants can be used to evaluate test suites or specs used for formal
  verification: each mutant represents a potential bug in the program, and
  stronger test suites and specifications should detect more mutants.

## Requirements

1. Gambit is written in Rust. You'll need to [install Rust and
   Cargo](https://www.rust-lang.org/tools/install) to build Gambit.
2. Gambit uses `solc`, the Solidity compiler, to generate mutants. You'll need
   to have a `solc` binary that is compatible with the project you are mutating (see
   the `--solc` option in `gambit mutate --help`)

## Installation

You can download prebuilt Gambit binaries for Mac and Linux from our
[releases](https://github.com/Certora/gambit/releases) page.

To build Gambit from source, clone [the Gambit repository](https://github.com/Certora/gambit) and run

```
cargo install --path .
```

from this repository's root. This will build Gambit and install it to a globally visible
location on your `PATH`.

You can also build gambit with `cargo build --release` from the root of this
repository.  This will create a `gambit` binary in `gambit/target/release/`
which you can manually place on your path or invoke directly (e.g., by calling
`path/to/gambit/target/release/gambit`).

## Usage

Gambit has two main commands: `mutate` and `summary`. `gambit mutate` is
responsible for mutating code, and `gambit summary` is a convenience command for
summarizing generated mutants in a human-readable way.

Running `gambit mutate` will invoke `solc`, so make
sure it is visible on your `PATH`. Alternatively, you can specify where Gambit can
find the Solidity compiler with the option `--solc path/to/solc`, or specify a
`solc` binary (e.g., `solc8.12`) with the option `--solc solc8.12`.

_**Note:**
All tests (`cargo test`) are currently run using `solc8.13`. Your tests may fail
  if your `solc` points at a different version of the compiler._

### Running  `gambit mutate` 

The `gambit mutate` command expects either a `--filename` argument or a `--json`
argument.  Using `--filename` allows you to specify a specific Solidity file to
mutate:

```bash
gambit mutate --filename file.sol
```

However, if you want to mutate multiple files or apply a more complex set of
parameters, we recommend using a configuration file via the `--json` option
instead:

```bash
gambit mutate --json gambit_conf.json
```

Run `gambit --help` for more information.

_**Note:**
All relative paths specified in a JSON configuration file are interpreted
to be relative to the configuration file's parent directory._

In the following section we provide examples of how to run Gambit using both
`--filename` and `--json`. We provide more complete documentation in the
[Configuration Files](#configuration-files) and [CLI-Options](#cli-options) sections below.

## Examples

Unless otherwise noted, examples use code from [benchmarks/](https://github.com/Certora/gambit/tree/master/benchmarks)
and are run from the root of the [Gambit repository](https://github.com/Certora/gambit).

### Example 1: Mutating a single file

To mutate a single file, use the `--filename` option (or `-f`), followed by the
file to mutate.

```bash
gambit mutate -f benchmarks/BinaryOpMutation/BinaryOpMutation.sol
```
<!-- Code output: using `pre` to avoid the Copy To Clipboard feature -->
<pre>
Generated 34 mutants in 0.69 seconds
</pre>

_**Note:**
The mutated file must be located within your current working directory or
one of its subdirectories. If you want to mutate code in an arbitrary directory,
use the `--sourceroot` option._

### Example 2: Mutating and downsampling

The above command produced 34 mutants which may be more than you need. Gambit
provides a way to randomly downsample the number of mutants with the
`--num_mutants` or `-n` option:

```bash
gambit mutate -f benchmarks/BinaryOpMutation/BinaryOpMutation.sol -n 3
```
<pre>
Generated 3 mutants in 0.15 seconds
</pre>

### Example 3: Viewing Gambit results
_**Note:**
This example assumes you've just completed Example 2._

Gambit outputs all of its results in `gambit_out`:

```bash
tree -L 2 gambit_out
```
<!-- Code output: using `pre` to avoid the Copy To Clipboard feature -->
<pre>
gambit_out
├── gambit_results.json
├── input_json
│   ├── BinaryOpMutation.sol_json.ast
│   └── BinaryOpMutation.sol_json.ast.json
├── mutants
│   ├── 1
│   ├── 2
│   └── 3
└── mutants.log
</pre>

See the [Results Directory](#results-directory) section for a detailed
explanation of this layout. The `gambit summary` command
pretty prints each mutant for easy inspection:

![The output of `gambit summary`](doc/gambit-summary.png)

By default `gambit summary` prints info on all mutants. If you are interested in
particular mutants you can specify a subset of mutant ids with the `--mids` flag.
For instance, `gambit summary --mids 3 4 5`  will only print info for mutant ids
3 through 5.


### Example 4: Specifying `solc` pass-through arguments
The Solidity compiler (`solc`) may need some extra information to successfully
run on a file or a project.  Gambit enables this with _pass-through arguments_
that, as the name suggests, are passed directly through to the `solc` compiler.

For projects that have complex dependencies and imports, you may need to:
* **Specify base paths**: To specify the Solidity [`--base-path`][basepath]
  argument, use `--solc_base_path`:

  ```bash
  gambit mutate --filename path/to/file.sol --solc_base_path base/path/dir
  ```

* **Specify remappings:** To indicate where Solidity should find libraries,
  use `solc`'s [import remapping][remapping] syntax with `--solc_remappings`:

  ```bash
  gambit mutate --filename path/to/file.sol \
    --solc_remappings @openzeppelin=node_modules/@openzeppelin @foo=node_modules/@foo
  ```

* **Specify allow paths:** To include additional allowed paths via `solc`'s
  [`--allow-paths`][allowed] argument, use `--solc_allow_paths`:

  ```bash
  gambit mutate --filename path/to/file.sol \
    --solc_allow_paths PATH1 --solc_allow_paths PATH2 ...
  ```

* **Specify include-path:** To make an additional source directory available
  to the default import callback via `solc`'s [--include-path][included] argument,
  use `--solc_include_path`:

  ```bash
  gambit mutate --filename path/to/file.sol --solc_include_path PATH
  ```

* **Use optimization:** To run the solidity compiler with optimizations
  (`solc`'s `--optimize` argument), use `--solc_optimize`:

  ```bash
  gambit mutate --filename path/to/file.sol --solc_optimize
  ```

[remapping]: https://docs.soliditylang.org/en/v0.8.17/path-resolution.html#import-remapping
[basepath]: https://docs.soliditylang.org/en/v0.8.17/path-resolution.html#base-path-and-include-paths
[allowed]: https://docs.soliditylang.org/en/v0.8.17/path-resolution.html#allowed-paths


<!-- ANCHOR: (gambit-config)= -->
### Example 5: The `--sourceroot`  option

Gambit needs to track the location of source files that it mutates within a
project: for instance, imagine there are files `foo/Foo.sol` and `bar/Foo.sol`.
These are separate files, and their path prefixes are needed to determine this.
Gambit addresses this with the `--sourceroot` option: the source root indicates
to Gambit the root of the files that are being mutated, and all source file
paths (both original and mutated) are reported relative to this source root.

_**Note:**
If Gambit encounters a source file that does not belong to the source root it
will print an error message and exit._

_When running `gambit mutate` with the `--filename` option,
source root defaults to the current working directory.
When running `gambit mutate` with the `--json` option,
source root defaults to the directory containing the configuration JSON._

Here are some examples of using the `--sourceroot` option.

1. From the root of the Gambit repository, run:

   ```bash
   gambit mutate -f benchmarks/BinaryOpMutation/BinaryOpMutation.sol -n 1
   cat gambit_out/mutants.log
   find gambit_out/mutants -name "*.sol"
   ```

   This should output the following:
   <!-- Code output: using `pre` to avoid the Copy To Clipboard feature -->
   <pre>
   Generated 1 mutants in 0.13 seconds
   1,BinaryOpMutation,benchmarks/BinaryOpMutation/BinaryOpMutation.sol,23:10, % ,*
   gambit_out/mutants/1/benchmarks/BinaryOpMutation/BinaryOpMutation.sol
   </pre>

   The first command generates a single mutant, and its source path is relative to `.`,
   the default source root. We can see that the reported paths in `mutants.log`,
   and the mutant file path in `gambit_out/mutants/1`, are the relative to this
   source root: `benchmarks/BinaryOpMutation/BinaryOpMutation.sol`

2. Suppose we want our paths to be reported relative to
   `benchmarks/BinaryOpMutation`. We can run

   ```bash
   gambit mutate -f benchmarks/BinaryOpMutation/BinaryOpMutation.sol -n 1 --sourceroot benchmarks/BinaryOpMutation
   cat gambit_out/mutants.log
   find gambit_out/mutants -name "*.sol"
   ```

   which will output:

   <!-- Code output: using `pre` to avoid the Copy To Clipboard feature -->
   <pre>
   Generated 1 mutants in 0.13 seconds
   1,BinaryOpMutation,BinaryOpMutation.sol,23:10, % ,*
   gambit_out/mutants/1/BinaryOpMutation.sol
   </pre>

   The reported filenames, and the offset path inside of
   `gambit_out/mutants/1/`, are now relative to the source root that we
   specified.

3. Finally, suppose we use a source root that doesn't contain the source file:

   ```bash
   gambit mutate -f benchmarks/BinaryOpMutation/BinaryOpMutation.sol -n 1 --sourceroot scripts
   ```
   This will try to find the specified file inside of `scripts`, and since it
   doesn't exist Gambit reports the error:

   <!-- Code output: using `pre` to avoid the Copy To Clipboard feature -->
   <pre>
   [ERROR gambit] [!!] Illegal Configuration: Resolved filename `/Users/USER/Gambit/benchmarks/BinaryOpMutation/BinaryOpMutation.sol` is not prefixed by the derived source root /Users/USER/Gambit/scripts
   </pre>

   Gambit prints an error and exits.

### Example 6: Running Gambit using a configuration file

To run gambit with a configuration file, use the `--json` argument:
```bash
gambit mutate --json benchmarks/config-jsons/test1.json
```

The configuration file is a JSON file containing the command line arguments for
`gambit` and additional configuration options:

```json
{
    "filename": "../10Power/TenPower.sol",
    "sourceroot": "..",
    "solc_remappings": [
        "@openzeppelin=node_modules/@openzeppelin"
    ],
}
```

In addition to specifying the command line arguments, you can list the specific
mutants that you want to apply, the specific functions you wish to mutate, and
more.  See the [`benchmark/config-jsons` directory][config-examples] for
examples.

_**Note:**
Any paths provided by the configuration file are resolved relative to the
configuration file's parent directory._

<!-- ANCHOR: (configuration-files)= -->
## Configuration Files
Configuration files allow you to save complex configurations and perform
multiple mutations at once. Gambit uses a simple JSON object format to store
mutation options, where each `--option VALUE` specified on the CLI is
represented as a `"option": VALUE` key/value pair in the JSON object.  Boolean
`--flag`s are enabled by storing them as true: `"flag": true`. For instance,
`--no_overwrite` would be written as `"no_overwrite": true`.

As an example, consider the command from Example 1:

```bash
gambit mutate -f benchmarks/BinaryOpMutation/BinaryOpMutation.sol
```

To execute this using a configuration file you would write the following to
`example-1.json` to the root of this repository and run `gambit mutate --json
example-1.json`

```json
{
  "filename": "benchmarks/BinaryOpMutation/BinaryOpMutation.sol"
}
```

Gambit also supports using multiple configurations in the same file: instead of
a single JSON object, your configuration file should contain an array of objects:

```json
[
    {
        "filename": "Foo.sol",
        "contract": "C",
        "functions": ["bar", "baz"],
        "solc": "solc8.12",
        "solc_optimize": true
    },
    {
        "filename": "Blip.sol",
        "contract": "D",
        "functions": ["bang"],
        "solc": "solc8.12"
        "mutations": [
          "binary-op-mutation",
          "swap-arguments-operator-mutation"
        ]
    }
]
```

This configuration file will perform all mutations on `Foo.sol`'s functions
`bar` and `baz` in the contract `C`, and only `binary-op-mutation` and
`swap-arguments-operator-mutation` mutations on the function `bang` in the
contract `D`.  Both will compile using the Solidity compiler version `solc5.12`.

### Paths in Configuration Files

Relative paths in a Gambit configuration file are _relative to the parent
directory of the configuration file_. So if the JSON file listed above was moved
to the `benchmarks/` directory the `"filename"` would need to be updated to
`BinaryOpMutation/BinaryOpMutation.sol`.

<!-- ANCHOR: (results-directory)= -->
## Results Directory

`gambit mutate` produces all results in an output directory (default:
`gambit_out`). Here is an example:

```bash
gambit mutate -f benchmarks/BinaryOpMutation/BinaryOpMutation.sol -n 5
tree gambit_out -L 2
```
<!-- Code output: using `pre` to avoid the Copy To Clipboard feature -->
<pre>
Generated 5 mutants in 0.15 seconds

gambit_out
├── gambit_results.json
├── input_json
├── mutants
│   ├── 1
│   ├── 2
│   ├── 3
│   ├── 4
│   └── 5
└── mutants.log

</pre>

This has the following structure:
+ `gambit_results.json`: a JSON file with detailed results
+ `input_json/`: intermediate files produced by `solc` that are used during mutation
+ `mutants/`: exported mutants. Each mutant is in its own directory named after
  its mutant ID (mid) 1, 2, 3, ...
+ `mutants.log`: a log file with all mutant information. This is similar to
  `results.json` but in a different format and with different information

<!-- ANCHOR: (cli-options)= -->
## CLI Options

 `gambit mutate` supports the following options; for a comprehensive list, run
 `gambit mutate --help`:


| Option                | Description                                                                                                                  |
| :-------------------- | :--------------------------------------------------------------------------------------------------------------------------- |
| `-o`, `--outdir`      | specify Gambit's output directory (defaults to `gambit_out`)                                                                 |
| `--no_overwrite`      | do not overwrite an output directory; if the output directory exists, print an error and exit                                |
| `-n`, `--num_mutants` | randomly downsample to a given number of mutants.                                                                            |
| `-s`, `--seed`        | specify a random seed. For reproducibility, Gambit defaults to using the seed `0`. To randomize the seed use `--random_seed` |
| `--random_seed`       | use a random seed. Note that this overrides any value specified by `--seed`                                                  |
| `--contract`          | specify a specific contract name to mutate; by default mutate all contracts                                                  |
| `--functions`         | specify one or more functions to mutate; by default mutate all functions                                                     |
| `--mutations`         | specify one or more mutation operators to use; only generates mutants that are created using the specified operators         |
| `--skip_validate`     | only generate mutants without validating them by compilation                                                                 |

Gambit also supports _pass-through arguments_, which are arguments that are
passed directly to the solidity compiler. All pass-through arguments are
prefixed with `solc_`:

| Option                | Description                                                                     |
| :-------------------- | :------------------------------------------------------------------------------ |
| `--solc_base_path`    | passes a value to `solc`'s `--base-path` argument                               |
| `--solc_include_path` | passes a value to `solc`'s `--include-path` argument                            |
| `--solc_remappings`   | passes a value to directly to `solc`: this should be of the form `prefix=path`. |
| `--solc_allow_paths`  | passes a value to `solc`'s `--allow-paths` argument                             |

## Mutation Operators
Gambit implements the following mutation operators

| Mutation Operator                    | Description                                              | Example                                        |
| ------------------------------------ | -------------------------------------------------------- | ---------------------------------------------- |
| **binary-op-mutation**               | Replace a binary operator with another                   | `a+b` -> `a-b`                                 |
| **unary-operator-mutation**          | Replace a unary operator with another                    | `~a` -> `-a`                                   |
| **require-mutation**                 | Alter the condition of a `require` statement             | `require(some_condition())` -> `require(true)` |
| **assignment-mutation**              | Replaces the right hand side of an assignment            | `x = foo();` -> `x = -1;`                      |
| **delete-expression-mutation**       | Replaces an expression with a no-op (`assert(true)`)     | `foo();` -> `assert(true);`                    |
| **if-cond-mutation**                 | Mutate the conditional of an `if` statement              | `if (C) {...}` -> `if (true) {...}`            |
| **swap-arguments-operator-mutation** | Swap the order of non-commutative operators              | `a - b` -> `b - a`                             |
| **elim-delegate-mutation**           | Change a `delegatecall()` to a `call()`                  | `_c.delegatecall(...)` -> `_c.call(...)`       |
| **function-call-mutation**           | **(Disabled)** Changes arguments of a function           | `add(a, b)` -> `add(a, a)`                     |
| **swap-arguments-function-mutation** | **(Disabled)** Swaps the order of a function's arguments | `add(a, b)` -> `add(b, a)`                     |

For more details on each mutation type, refer to the [full documentation](https://docs.certora.com/en/latest/docs/gambit/gambit.html#mutation-types).

<!-- SUPPRESS -->
## Contact
If you have ideas for interesting mutations or other features,
we encourage you to make a PR or [email](mailto:chandra@certora.com) us.

## Credits
We thank
[Oliver Flatt](https://www.oflatt.com/) and
[Vishal Canumalla](https://homes.cs.washington.edu/~vishalc/)
for their excellent contributions to an earlier prototype of Gambit.

<!-- END SUPPRESS -->

[config-examples]: https://github.com/Certora/gambit/blob/master/benchmarks/config-jsons/
[test6]: https://github.com/Certora/gambit/blob/master/benchmarks/config-jsons/test6.json
