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

}

