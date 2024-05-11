## DISCLAIMER
This package has not been tested yet. Use only for inspiration for now.

## Overview
This blueprint enables advanced staking of resources. Staking rewards are distributed periodically.

### Advantages
The 3 main advantages over simple OneResourcePool staking that are accomplished are:
- Staking reward can be a token different from the staked token.
- Staked tokens can be locked (e.g. for voting or to reward not selling).
- An unstaking delay can be set (is technically also possible using the OneResourcePool).

To accomplish this, users now stake their tokens to a staking ID. The staked tokens are then held by the staking component:
- Rewards are claimed through the component, which can distribute any token as a reward.
- The component can easily lock these tokens.
- Unstaking is done by requesting an unstaking receipt, which can be redeemed through the component after a set delay, providing an unstaking delay.

### Disadvantages
This NFT staking ID approach has some disadvantages over simple OneResourcePool staking:
- Wallet display of staked tokens is more difficult, as staked amounts are stored by an NFT (staking ID). Ideally, users need to use some kind of front-end to see their staked tokens. Alternatively, you could provide the staker with a placeholder token, so they can easily see how much they've staked.
- Staking rewards are distributed periodically, not continuously.
- User needs to claim rewards manually. Though this could be automated in some way.
- Staked tokens are not liquid, making it impossible to use them in traditional DEXes. Though they are transferable to other user's staking IDs, so a DEX could be built on top of this system. This way, liquidity could be provided while still earning staking fees.
- It is more complex to set up and manage.

## Implementation
Within the lib.rs file, you will find all methods with accompanying comments.

To make it easier to understand, here is a general overview of the blueprint's working:

The instantiator of the staking component can add stakable tokens, and set rewards for staking them, or locking them. This information is stored within the staking component.

All staking revolves around the Staking ID. A user can create a staking ID at any time. The ID contains a HashMap, with entries of all staked tokens. The entries contain information about the amount of staked tokens, and until when they are locked. A user can stake tokens to it by presenting their ID.

The staking component records how many tokens are staked in total for all stakable tokens. When a period ends, it calculates how many tokens should be rewarded per staked stakable token for that period. It does this by dividing the total period reward by the total amount staked.

A user can then come back and claim their staking rewards for the periods that have passed since they have last done so. To do this, they present their staking ID.

Provided the tokens aren't locked, unstaking is done by again presenting the staking ID. This results in an unstaking receipt, which can, after the unstaking delay has passed, be redeemed for the unstaked tokens.

## Setup
__Required knowledge:__
To set up a staking component, you will need to know how to build (for now, since no package is deployed already) and deploy scrypto packages and write and send transaction manifests. In the future, information on how to do this will be available here. For now, please refer to Radix' documentation: https://docs.radixdlt.com/docs

Though, if you have trouble with writing Transaction Manifests, using https://instruct-stokenet.radixbillboard.com/ is heavily recommended! It will automatically detect what arguments a chosen method expects.

__Setup:__
Setting up a staking component is fairly easy. First, clone this repo and build the package, and deploy it to ledger (in the future, a package will be deployed on main net and stokenet already for you to use).

### Instantiation
Now the package is deployed to ledger, it's time to instantiate your own Staking component by calling the new method, which looks like this:

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
- The controller argument is the ResourceAddress corresponding to the desired Owner Role. In other words, holding that resource gives access to the OWNER role in the Staking Component.
- The rewards are argument is a bucket of fungible resources you wish to award for staking (or locking) tokens.
- The period_interval argument is the amount of days every reward cycle has.
- The name and symbol argument influence your component's metadata.
- The dao_controlled argument influences the amount of influence the OWNER has. If the owner badge is held by a centralized entity, setting this value to false stops the owner from locking staked tokens. If it's set to true, the owner badge can be used to lock staked - tokens (for instance, if a staking id is used to vote).
- The max_unstaking_delay sets an upper limit to the delay between unstaking and being able to redeem your unstaked tokens. This delay can be set by the component's owner, and this maximum value provides a guarantee, so the owner can not lock all staked tokens indefinitely.

### Adding stakables
When the component is deployed, you can interact with it. One of the first first methods you might want to call is the add_stakable method, which enables staking of a chosen resource:

```rust
pub fn add_stakable(&mut self, address: ResourceAddress, reward_amount: Decimal, lock: Lock)
```

- The address argument is the address of the resource that becomes stakable.
- The reward_amount is the amount of tokens from the reward vault you want to reward every reward cycle.
- The lock argument is a Lock struct, which specifies whether the reward for locking this stake, and looks like:
```rust
pub struct Lock {
    pub payment: Decimal,
    pub duration: i64,
}
```
- The payment argument is the amount of tokens rewarded for locking (per locked token)
- The duration argument is the amount of days the stake will be locked

### Creating a staking ID
To stake, a user needs to create a staking ID by calling the ``create_id`` method.

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
