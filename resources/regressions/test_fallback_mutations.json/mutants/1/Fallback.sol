// SPDX-License-Identifier: GPL-3.0-only

pragma solidity ^0.8.13;

contract Fallback {
    function checkArgumentsAreReplaced(
        uint256 num,
        address addr,
        bool b
    /// ExpressionValueReplacement(`num == 0` |==> `true`) of: `if (num == 0) {`
    ) public returns (uint256) {
        if (true) {
            return 0;
        } else {
            return checkArgumentsAreReplaced(num - 1, addr, !b);
        }
    }
}
