// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >0.7.0;
pragma experimental ABIEncoderV2;

contract SwapArgumentsOperatorMutation {
    function mySubtraction(uint256 x, uint256 y) public pure returns (uint256) {
	return x - y;
    }
    
    function myDivision(uint256 x, uint256 y) public pure returns (uint256) {
	return x / y;
    }
    
    function myModulo(uint256 x, uint256 y) public pure returns (uint256) {
	/// BinaryOpMutation(`%` |==> `*`) of: `return x % y;`
	return x*y;
    }
    
    function myExponentiation(uint256 x, uint256 y) public pure returns (uint256) {
	return x ** y;
    }
    
    function myGT(uint256 x, uint256 y) public pure returns (bool) {
	return x > y;
    }
    
    function myLT(uint256 x, uint256 y) public pure returns (bool) {
	return x < y;
    }
    
    function myGE(uint256 x, uint256 y) public pure returns (bool) {
	return x >= y;
    }
    
    function myLE(uint256 x, uint256 y) public pure returns (bool) {
	return x <= y;
    }

    function mySAL(uint256 x, uint256 y) public pure returns (uint256) {
	return x << y;
    }

    function mySAR(uint256 x, uint256 y) public pure returns (uint256) {
	return x >> y;
    }
}
