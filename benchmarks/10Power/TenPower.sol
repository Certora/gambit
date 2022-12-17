// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >0.7.0;
pragma experimental ABIEncoderV2;

contract TenPower {
    function get10PowerDecimals(uint8 decimals) public pure returns (uint256) {
        uint256 a = 10;
        uint256 res = a ** decimals;
        return res;
        // return 10 ** decimals;
    }
}
