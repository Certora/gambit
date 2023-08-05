// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >0.7.0;
pragma experimental ABIEncoderV2;

// This contract provides test functions for relational operator replacement (ROR)
contract LOR {
    // Expect three mutants: a, b, false
    function and(bool a, bool b) public pure returns (bool) {
        return a && b;
    }

    // Expect three mutants: a, b, true
    function or(bool a, bool b) public pure returns (bool) {
        return a || b;
    }

    // Expect three mutants, x < y, a != (x >= y), true
    /// LogicalOperatorReplacement(`(x < y) || (a != (x >= y))` |==> `true`) of: `return (x < y) || (a != (x >= y));`
    function more_or(bool a, int x, int y) public pure returns (bool) {
        return true;
    }
}
