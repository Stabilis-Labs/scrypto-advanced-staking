## USE OF BLUEPRINT
Distribute a specific token as a reward for staking tokens of the admin's choosing every chosen interval.

## BASIC WORKING OF BLUEPRINT

Instantiation creates a staking component, which distributes a specific token as a reward for staking tokens of the admin's choosing every chosen interval.

The admin can call a method to add stakables to the component, and choose how many rewards to hand out every period.

The user can generate a soulbound id (nft) which records how much of every resource they have staked and when they have last claimed their staking rewards.

Then there's a publicly callable method that checks whether a staking period has passed, calculates the amount of rewards that should be handed out per staked token and records that somewhere. 

If the user then comes back a few periods later and calls the method to claim rewards it is calculated how much they are owed and handed to them.

## DISCLAIMER

More documentation will follow.

No testing or comments are currently present.
