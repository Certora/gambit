// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >0.7.0;
pragma experimental ABIEncoderV2;

// This contract provides test functions for Bitwise Operator Replacement (BOR)
contract BOR {
    // Expect 1 mutants:
    // a & b;
    function bw_or(int256 a, int256 b) public pure returns (int256) {
        return a | b;
    }

    // Expect 1 mutants:
    // a | b;
    function bw_and(int256 a, int256 b) public pure returns (int256) {
        return a & b;
    }

    // Expect 1 mutants:
    // a | b;
    function bw_xor(int256 a, int256 b) public pure returns (int256) {
        return a ^ b;
    }
}
