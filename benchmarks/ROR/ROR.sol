// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >0.7.0;
pragma experimental ABIEncoderV2;

// This contract provides test functions for relational operator replacement (ROR)
contract ROR {
    function less(uint256 x, uint256 y) public pure returns (bool) {
        return x < y;
    }

    function less_equal(uint256 x, uint256 y) public pure returns (bool) {
        return x <= y;
    }

    function more(uint256 x, uint256 y) public pure returns (bool) {
        return x > y;
    }

    function more_equal(uint256 x, uint256 y) public pure returns (bool) {
        return x >= y;
    }

    function equal_ord(uint256 x, uint256 y) public pure returns (bool) {
        return x == y;
    }

    function equal_not_ord(bool x, bool y) public pure returns (bool) {
        return x == y;
    }

    function not_equal_ord(uint256 x, uint256 y) public pure returns (bool) {
        return x != y;
    }

    function not_equal_not_ord(bool x, bool y) public pure returns (bool) {
        return x != y;
    }
}
