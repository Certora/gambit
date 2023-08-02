// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >0.7.0;
pragma experimental ABIEncoderV2;

// This contract provides test functions for Arithmetic operator replacement
contract AOR {
    // Expect 4 mutants:
    // a - b
    // a * b
    // a / b
    // a % b
    function plus(int256 a, int256 b) public pure returns (int256) {
        return a + b;
    }

    // Expect 4 mutants:
    // a + b
    // a * b
    // a / b
    // a % b
    /// ArithmeticOperatorReplacement(`-` |==> `*`) of: `return a - b;`
    function minus(int256 a, int256 b) public pure returns (int256) {
        return a * b;
    }

    // Expect 4 mutants:
    // a - b
    // a * b
    // a / b
    // a % b
    function times_with_parens(
        int256 a,
        int256 b
    ) public pure returns (int256) {
        return ((a)) * b;
    }

    // Expect 5 mutants:
    // a + b
    // a - b
    // a / b
    // a ** b
    // a % b
    function unsigned_times_with_parens(
        uint256 a,
        uint256 b
    ) public pure returns (uint256) {
        return ((a)) * b;
    }

    // Expect 5 mutants:
    // a + b
    // a - b
    // a / b
    // a * b
    // a % b

    function power(uint256 a, uint256 b) public pure returns (uint256) {
        return a ** b;
    }
}
