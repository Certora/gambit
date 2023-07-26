// SPDX-License-Identifier: GPL-3.0-only
pragma solidity ^0.8.13;

contract B {
    function setVars(uint _num) public payable {}
}

contract A {
    uint public num;
    address public sender;
    uint public value;
    bool public delegateSuccessful;
    bytes public myData;

    function setVars(address _contract) public payable {
        (bool success, ) = _contract.delegatecall(
            abi.encodeWithSignature("setVars(uint256)", 1)
        );
        require(success, "Delegatecall failed");
    }
}
