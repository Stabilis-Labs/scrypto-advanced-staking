# Scrypto Advanced Staking
by Stabilis Labs

__DISCLAIMER__: THIS PACKAGE HAS NOT BEEN TESTED YET. USE ONLY FOR INSPIRATION NOW.

# Overview
This package enables advanced staking of resources. This is done by staking tokens to a Staking ID. This ID records how many tokens are staked, and whether they are locked. By showing this ID, users can stake, unstake, lock their tokens and claim staking rewards. Assignment of rewards is done periodically.

### Advantages
The 3 main advantages over simple OneResourcePool staking that are accomplished are:
1. Staking reward can be a token different from the staked token.
2. Staked tokens can be locked (e.g. for voting or to reward not selling).
3. An unstaking delay can be set (is technically also possible using the OneResourcePool).

### Disadvantages
This NFT staking ID approach has some disadvantages over simple OneResourcePool staking:
1. Wallet display of staked tokens is more difficult, as staked amounts are stored by an NFT (staking ID). Ideally, users need to use some kind of front-end to see their staked tokens. Alternatively, you could provide the staker with a placeholder token, so they can easily see how much they've staked.
2. Staking rewards are distributed periodically, not continuously.
3. User needs to claim rewards manually. Though this could be automated in some way.
4. Staked tokens are not liquid, making it impossible to use them in traditional DEXes. Though they are transferable to other user's staking IDs, so a DEX could be built on top of this system. This way, liquidity could be provided while still earning staking fees.
5. It is more complex to set up and manage.

# Implementation

## Required knowledge
To set up a staking component, you will need to know how to build (for now, since no package is deployed already) and deploy scrypto packages and write and send transaction manifests. In the future, information on how to do this will be available here. For now, please refer to Radix' documentation: https://docs.radixdlt.com/docs

Though, if you have trouble with writing Transaction Manifests, using https://instruct-stokenet.radixbillboard.com/ is heavily recommended! It will automatically detect what arguments a chosen method expects.

## Setup
Setting up a staking component is fairly easy. First, clone this repo and build the package, and deploy it to ledger (in the future, a package will be deployed on main net and stokenet already for you to use).

### Instantiation
Now the package is deployed to ledger, it's time to instantiate your own Staking component by calling the ``new`` function, which looks like this:

```rust
pub fn new(
            controller: ResourceAddress,
            rewards: FungibleBucket,
            period_interval: i64,
            name: String,
            symbol: String,
            dao_controlled: bool,
            max_unstaking_delay: i64,
        ) -> Global<Staking>
```
- The ``controller`` argument is the ResourceAddress corresponding to the desired Owner Role. In other words, holding that resource gives access to the OWNER role in the Staking Component.
- The ``rewards`` are argument is a bucket of fungible resources you wish to award for staking (or locking) tokens.
- The ``period_interval`` argument is the amount of days every reward cycle has.
- The ``name`` and ``symbol`` arguments influence your component's metadata.
- The ``dao_controlled`` argument influences the amount of influence the OWNER has. If the owner badge is held by a centralized entity, setting this value to false stops the owner from locking staked tokens. If it's set to true, the owner badge can be used to lock staked tokens (for instance, if a staking id is used to vote).
- The ``max_unstaking_delay`` sets an upper limit to the delay between unstaking and being able to redeem your unstaked tokens. This delay can be set by the component's owner, and this maximum value provides a guarantee, so the owner can not lock all staked tokens indefinitely.

### Adding stakables
When the component is deployed, you can interact with it. One of the first first methods you might want to call is the ``add_stakable`` method, which enables staking of a chosen resource:

```rust
pub fn add_stakable(&mut self, address: ResourceAddress, reward_amount: Decimal, lock: Lock)
```

- The ``address`` argument is the address of the resource that becomes stakable.
- The ``reward_amount`` is the amount of tokens from the reward vault you want to reward every reward cycle.
- The ``lock argument`` is a Lock struct, which specifies whether the reward for locking this stake, and looks like:
```rust
pub struct Lock {
    pub payment: Decimal,
    pub duration: i64,
}
```
- The payment argument is the amount of tokens rewarded for locking (per locked token)
- The duration argument is the amount of days the stake will be locked

If you don't wish to add locking capability, simply set both to 0.

__IMPORTANT:__ This method requires the Owner role, so be sure to show proof of your owner badge in the Manifest.

### Creating a staking ID
To stake, a user needs to create a staking ID by calling the ``create_id`` method, which does not require any arguments, and will return a Bucket with a Staking ID.

### Stake / staking
To stake to a Staking ID, the ``stake`` method is called, which looks like:

```rust
pub fn stake(&mut self, stake_bucket: Bucket, id_proof: NonFungibleProof)
```

- The ``stake_bucket`` argument is a bucket of either the stakable tokens, or a stake transfer receipt (which is a receipt that can be used to transfer stake from one ID to another)
- The ``id_proof`` argument is a NonFungibleProof of the Staking ID, to prove the user is in possession of it.

### Unstaking
Unstaking consists of two steps:
1. Requesting unstake and receiving an unstaking receipt / stake transfer receipt.
2. Redeeming unstaked tokens using the unstaking receipt.

The first step can be done through calling the ``start_unstake`` method:

```rust
pub fn start_unstake(
            &mut self,
            id_proof: NonFungibleProof,
            address: ResourceAddress,
            amount: Decimal,
            stake_transfer: bool,
        ) -> Bucket
```

- The ``id_proof`` argument is a NonFungibleProof of the Staking ID, to prove the user is in possession of it.
- The ``address`` argument is the ResourceAddress of the token you wish to unstake
- The ``amount`` argument is the amount of tokens you wish to unstake
- The ``stake_transfer`` argument is a bool that decides whether you receive an unstaking receipt or a stake transfer receipt. The former can be redeemed in return for your staked tokens after the unstaking delay, while the latter can be immediately used to transfer your staked tokens to another Staking ID.
- The method returns a Bucket containing an unstaking receipt or a stake transfer receipt.

To redeem an unstaking receipt, the ``finish_unstake`` method is called:

```rust
pub fn finish_unstake(&mut self, receipt: Bucket) -> Bucket
```

- The ``receipt`` argument is an unstaking receipt (if the unstaking delay has not yet passed, the method will fail)
- The Bucket returned contains the unstaked tokens.

### Locking stake
Locking stake can be done through the ``lock_stake`` method:

```rust
pub fn lock_stake(&mut self, address: ResourceAddress, id_proof: NonFungibleProof) -> FungibleBucket
```

- The ``address`` argument is the ResourceAddress of the token you wish to lock
- The ``id_proof`` argument is a NonFungibleProof of the Staking ID, to prove the user is in possession of it.
- The returned Bucket contains the locking rewards.

### Claiming rewards
Claiming accrued rewards is done by calling the ``update_id`` method:

```rust
pub fn update_id(&mut self, id_proof: NonFungibleProof) -> FungibleBucket
```

- The ``id_proof`` argument is a NonFungibleProof of the Staking ID you wish to claim rewards for.
- A FungibleBucket of rewards is returned.

__IMPORTANT__: The ``max_claim_delay`` parameter of the system determines the amount of previous periods you can still claim rewards from. By default, it's set to 5, but it can be altered by the component owner.

### Admin methods
To update the system, a plethora of admin methods exists . Please refer to the blueprint for these. They are very simple, but all require proof of the owner badge, so be sure to include this in the manifest.

## License

This package is released under modified MIT License.

    Copyright 2024 Stabilis Labs

    Permission is hereby granted, free of charge, to any person obtaining a copy of
    this software and associated documentation files (the "Software"), to deal in
    the Software for non-production informational and educational purposes without
    restriction, including without limitation the rights to use, copy, modify,
    merge, publish, distribute, sublicense, and to permit persons to whom the
    Software is furnished to do so, subject to the following conditions:

    This notice shall be included in all copies or substantial portions of the
    Software.

    THE SOFTWARE HAS BEEN CREATED AND IS PROVIDED FOR NON-PRODUCTION, INFORMATIONAL
    AND EDUCATIONAL PURPOSES ONLY.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
    FOR A PARTICULAR PURPOSE, ERROR-FREE PERFORMANCE AND NONINFRINGEMENT. IN NO
    EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES,
    COSTS OR OTHER LIABILITY OF ANY NATURE WHATSOEVER, WHETHER IN AN ACTION OF
    CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE
    SOFTWARE OR THE USE, MISUSE OR OTHER DEALINGS IN THE SOFTWARE. THE AUTHORS SHALL
    OWE NO DUTY OF CARE OR FIDUCIARY DUTIES TO USERS OF THE SOFTWARE.
