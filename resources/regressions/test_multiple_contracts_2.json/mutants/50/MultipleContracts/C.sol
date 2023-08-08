// SPDX-License-Identifier: GPL-3.0-only

pragma solidity ^0.8.13;

library Utils {
    function getarray(address[] memory c, address e) internal pure {
        assert(c[0] == e);
    }

    function add(int8 a, int8 b) public pure returns (int8) {
        return a + b;
    }
}

contract C {
    function foo() external view returns (address[] memory) {
        address[] memory a = new address[](1);
        a[0] = msg.sender;
        return a;
    }

    function get10PowerDecimals(uint8 decimals) public pure returns (uint256) {
        /// ArithmeticOperatorReplacement(`**` |==> `%`) of: `uint256 res = a ** decimals;`
        uint256 a = 10;
        uint256 res = a % decimals;
        return res;
    }

    function getarray(address[] memory c, address e) public pure {
        assert(c[0] == e);
    }

    function callmyself() external view {
        address[] memory b = this.foo();
        Utils.getarray(b, address(this));
    }

    function add(int8 c, int8 d) public pure returns (int8) {
        return c + d;
    }
}
