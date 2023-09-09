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
  


  ## Checking RTDs are Up To Date


  In addition to translating this document to the RTD format,
  `generate_rtd_markdown.py` also adds the md5 checksum of the original
  `README.md` contents to an HTML comment in the translated `gambit.md`:

  ```
  <\!-- signature: CHECKSUM --\> 
  ```

  You can check to ensure that the current version of `docs/gambit/gambit.md` in
  the Certora Documentation repo is up to date with the version of the
  `README.md` in your working tree by running

  To check that this file and RTD Gambit docs are in sync, run:

  ```
  python scripts/check_rtd_docs_upt_to_date.py
  ```
  
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

     We don't have access tho this here, so I've implemented a simple system,
     where all notes begin with a line containing:

     ```markdown
     _**Note:**
     ```

     and end with a line containing only:

     ```markdown
     _
     ```

     So, a full note would look like:

     ```markdown
     _**Note:**
     This is a note. The opening tag is on its own line, and the closing italic
     is on its own line. This is to make parsing easy, and to keep diffs minimal!
     ```
     
-->

<!-- END SUPPRESS -->

<!-- EMIT:
<\!--
  WARNING: AUTO_GENERATED DOCUMENTATION

  The following documentation is automatically generated from the Gambit
  README.md located at https://github.com/Certora/Gambit/README.md. Please view
  this document for instructions on producing this file.
--\>
-->
# Gambit: Mutant Generation for Solidity

Gambit is a state-of-the-art mutant generation system for Solidity.  By applying
predefined syntax transformations called _mutation operators_ (for example,
convert `a + b` to `a - b`) to a Solidity program's source code, Gambit
generates variants of the program called _mutants_.
Mutants are used to evaluate a test suite or a specification: each mutant
represents a potential bug in the program, and stronger test suites and
specifications should detect more mutants as faulty.

## Requirements

1. Gambit is written in Rust. You'll need to [install Rust and
   Cargo](https://www.rust-lang.org/tools/install) to build Gambit.
2. Gambit uses the `solc` Solidity compiler to validate generated mutants. By
  default Gambit looks for `solc` on `$PATH`. Users can specify a particular
  `solc` executable with the `--solc` option, or disable validation entirely
  with `gambit mutate --skip_validate` (see `gambit mutate --help` for more
  details).

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
repository.  This will create the `target/release/gambit` binary which you can
manually place on your path or invoke directly.

## Usage

Gambit has two main commands: `mutate` and `summary`. The `mutate` command is
responsible for mutating code. The `summary` command allows the user to get
a high level summary of the results of an execution of `gambit mutate`.


<!-- SUPPRESS -->
## Testing

Gambit has _unit tests_ and _regression tests_. Run unit tests with `cargo
test`.

_**Note:**
All unit tests (`cargo test`) are currently run using `solc8.13`. Tests may fail
if `solc` points at a different version of the compiler._

Run regression tests with `scripts/run_regressions.sh`.  This script runs
`gambit mutate` on all configuration files in `benchmarks/config-jsons` and
compares the output against the expected output in `resources/regressions`.

_**Note:**
To update regression tests (e.g., in case of new test cases, new mutation
operators, altered mutation operators, etc), use the
`scripts/make_regressions.sh` script._

<!-- END SUPPRESS -->

### The  `mutate` command

The `mutate` command expects a filename `gambit mutate file.sol` or a
configuration file `gambit mutate --json gambit_conf.json`. The `mutate` command
does the following:

1. **Parse:** Gambit begins by parsing the specified Solidity files provided on
   command line or in the configuration file

2. **Function filters:** The `mutate` command provides the `--functions` and
  `--contract` filters to allow users to filter which functions should be
  mutated. When `--functions` is specified, Gambit will only mutate functions
  with a name contained in the provided list of functions. When `--contract` is
  specified, Gambit will only mutate functions within the specified contract. If
  neither option is specified, Gambit will mutate all functions.

3. **Mutation:** Next, Gambit recursively visits the body of each function
   retained in (2) and applies the mutation operators specified by the user;
   if no mutation operators were specified then Gambit uses a default set of
   mutation operators.

4. **Validation:** By default Gambit will _validate_ each
   generated mutant by compiling it with the `solc` compiler. If compilation
   fails Gambit will not export the mutant. Validation can be skipped with the
   `--skip_validate` option. To log invalidated mutants, use the `--log_invalid`
   option.

5. **Down sampling:** If the user provides the `--num_mutants n` argument,
   Gambit will randomly down sample to `n` mutants.
  
6. **Write to disk:** After all mutants are generated, validated, and optionally
   down sampled, the `mutate` writes the results to disk. This includes 
   as well as specify several

#### Specifying Import Paths and Remappings

Gambit resolves imports while parsing, and this requires that you specify any
import paths and remappings that you would pass to `solc`.

Instead of `solc`'s `--base-name` and `--input-path` arguments, Gambit uses
a simpler scheme and replaces both of these with `--import_path` (`-I`). For instance,
if the `solc` invocation is `solc C.sol --base-name . --input-path modules` ,
then the Gambit invocation becomes `gambit mutate C.sol -I . -I modules`.

Remappings are specified with the `--import_map` (`-m`) argument. If the `solc`
invocation is `solc C.sol @openzeppelin=node_modules/@openzeppelin`, then the
Gambit invocation becomes `gambit mutate C.sol -m
@openzeppelin=node_modules/@openzeppelin`.

#### Performing Mutant Validation

Gambit uses provided import paths and import remappings to invoke `solc`. For
instance, if you invoke `gambit mutate C.sol -I A/ -I B/ -I C/ -m @x=y/@x`, then
Gambit will validate a generated mutant by calling
`solc MutatedC.sol --base-path A/ --include-path B/ --include-path C/ @x=y/@x`.
If you need to specify a solc `--allow-paths` argument, use the `mutate`
command's `--solc_allow_paths` argument.

### The `summary` command

The `summary` command allows the user to see a summary of a `mutate` run:

<pre>
$ gambit mutate benchmarks/Ops/AOR/AOR.sol
Generated 27 mutants in 0.41 seconds

$ gambit summary

STD:      5 ( 18.52%)
AOR:     22 ( 81.48%)
---------------------
TOT:     27 (100.00%)
</pre>

To print the diffs of specific mutants, pass the `--mids` option:

<pre>
$ gambit summary --mids 1 2

             === Mutant ID: 1 [StatementDeletion] ===

--- original
+++ mutant
@@ -9,8 +9,9 @@
     // a * b
     // a / b
     // a % b
+    /// StatementDeletion(`return a + b` |==> `assert(true)`) of: `return a + b;`
     function plus(int256 a, int256 b) public pure returns (int256) {
-        return a + b;
+        assert(true);
     }

     // Expect 4 mutants:

Path: mutants/1/benchmarks/Ops/AOR/AOR.sol


             === Mutant ID: 2 [ArithmeticOperatorReplacement] ===

--- original
+++ mutant
@@ -9,8 +9,9 @@
     // a * b
     // a / b
     // a % b
+    /// ArithmeticOperatorReplacement(`+` |==> `-`) of: `return a + b;`
     function plus(int256 a, int256 b) public pure returns (int256) {
-        return a + b;
+        return a - b;
     }

     // Expect 4 mutants:

Path: mutants/2/benchmarks/Ops/AOR/AOR.sol
</pre>

Pass the `--short` option to print a shorter summary of each mutant:

<pre>
$ gambit summary --mids 1 2 3 4 5 --short
(1) STD [mutants/1/benchmarks/Ops/AOR/AOR.sol@13:9] return a + b -> assert(true)
(2) AOR [mutants/2/benchmarks/Ops/AOR/AOR.sol@13:18] + -> -
(3) AOR [mutants/3/benchmarks/Ops/AOR/AOR.sol@13:18] + -> *
(4) AOR [mutants/4/benchmarks/Ops/AOR/AOR.sol@13:18] + -> /
(5) AOR [mutants/5/benchmarks/Ops/AOR/AOR.sol@13:18] + -> %
</pre>

_**Note:**
The `summary` command is currently experimental, and its output and interface
may change in future releases._

## Examples

In this section we provide examples of how to run Gambit.  We provide more
complete documentation in the [Configuration Files](#configuration-files) and
[CLI-Options](#cli-options) sections below.  Unless otherwise noted, examples
use code from
[benchmarks/](https://github.com/Certora/gambit/tree/master/benchmarks) and are
run from the root of the [Gambit repository](https://github.com/Certora/gambit).

### Example 1: Mutating a single file

To mutate a single file, call `gambit mutate` with the filename as an argument:

```bash
gambit mutate -f benchmarks/Ops/AOR/AOR.sol
```
<!-- Code output: using `pre` to avoid the Copy To Clipboard feature -->
<pre>
Generated 27 mutants in 0.42 seconds
</pre>

If the mutated file is not located in your current working directory (or one of
its subdirectories), you will need to specify an import path. Running:


```
mkdir tmp
cd tmp
gambit mutate ../benchmarks/Ops/AOR/AOR.sol
```
<!-- Code output: using `pre` to avoid the Copy To Clipboard feature -->
<pre>
Error: File Not In Import Paths
   Could not mutate file /Users/benku/Gambit/benchmarks/Ops/AOR/AOR.sol:
   File could not be resolved against any provided import paths.
   Import Paths: ["/Users/benku/Gambit/tmp"]
</pre>

By specifying an import path that contains the mutated file with `-I ..` ,
Gambit is able to resolve the provided filename.

```
gambit mutate ../benchmarks/Ops/AOR/AOR.sol -I ..
```
<!-- Code output: using `pre` to avoid the Copy To Clipboard feature -->
<pre>
Generated 27 mutants in 0.42 seconds
</pre>

### Example 2: Mutating and downsampling

The above command produced 34 mutants which may be more than you need. Gambit
provides a way to randomly downsample the number of mutants with the
`--num_mutants` or `-n` option:

```
gambit mutate benchmarks/Ops/AOR/AOR.sol --num_mutants 3
```
<!-- Code output: using `pre` to avoid the Copy To Clipboard feature -->
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
