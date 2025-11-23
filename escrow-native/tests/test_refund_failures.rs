// TEST 1: Refund by Non-Maker (Wrong Signer)
// Test when someone other than the maker tries to refund

// TEST 2: Wrong Escrow ID
// Test when trying to refund with wrong escrow ID

// TEST 3: Vault is Empty (Already Refunded)
// Test when trying to refund an already-refunded escrow

// TEST 4: Invalid Mint A
// Test when wrong mint is provided for Token A in refund

// TEST 5: Token Account Doesn't Belong to Maker
// Test when token account is not owned by maker

// TEST 6: Escrow Account Not Owned by Program
// Test when escrow account is not owned by program

// TEST 7: Vault PDA Mismatch
// Test when vault account is not the correct PDA

// TEST 8: Token Account Mint Mismatch
// Test when token account mint doesn't match expected mint
