pragma solidity ^0.5.12;

library Utils {
    function getarray(address[] memory c, address e) internal view {
        assert(c[0] == e);    
   
    }

}
contract C {
   
    function foo() external view returns (address[] memory)  {
        address[] memory a = new address[](1);
        a[0] = msg.sender;
        return a;
    }

    function get10PowerDecimals(uint8 decimals) public pure returns (uint256) {
        uint256 a = 10;
        uint256 res = a ** decimals;
        return res;
    }

     function getarray(address[] memory c, address e) public view {
        assert(c[0] == e);    
   
    }
   
    function callmyself() external {
        address[] memory b = this.foo();
        Utils.getarray(b,address(this));
    }
   
   
	// TODO: Add more checks. Return more than one element, make sure it's not negative due to SMT model and not causing an overflow somewhere..
   
}