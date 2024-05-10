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
- Wallet display of staked tokens is more difficult, as staked amounts are stored by an NFT (staking ID). Ideally, users need to use some kind of front-end to see their staked tokens.
- Staking rewards are distributed periodically, not continuously.
- User needs to claim rewards manually. Though this could be automated in some way.
- Staked tokens are not liquid, making it impossible to use them in traditional DEXes. Though they are transferable to other user's staking IDs, so a DEX could be built on top of this system. This way, liquidity could be provided while still earning staking fees.
- It is more complex to set up and manage.

## Implementation
Within the lib.rs file, you will find all methods with accompanying comments.

To make it easier to understand, here is a general overview of the blueprint's working:

The instantiator of the staking component can add stakable tokens, and set rewards for staking them, or locking them. This information is stored within the staking component.

All staking revolves around the Staking ID. A user can create a staking ID at any time. The ID contains a list of the amount of token a user has staked. A user can stake tokens to it by presenting their ID.

The staking component records how many tokens are staked in total for all stakable tokens. When a period ends, it calculates how many tokens should be rewarded per staked stakable token for that period. It does this by dividing the total period reward by the total amount staked.

A user can then come back and claim their staking rewards for the periods that have passed since they have last done so. To do this, they present their staking ID.

Providing the tokens aren't locked, unstaking is done by again presenting the staking ID. This results in an unstaking receipt, which can, after the unstaking delay has passed, be redeemed for the unstaked tokens.

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
