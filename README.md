# Blockchain

Simple implementation of Blockchain in Rust. The user can communicate with the blockchain
via CLI.

## Basic commands

- Create **Wallet**:

  ```bash
  blockchain create-wallet
  # Returns address of the wallet
  ```

- Creates a **Blockchain** and issues 10 coins to the wallet:

  ```bash
  blockchain create $WALLET_ADDRESS
  ```

- Get balance:

  ```bash
  blockchain get-balance $WALLET_ADDRESS
  ```

- Send coins:

  ```bash
  blockchain send $FROM_WALLET $TO_WALLET $AMOUNT
  ```
