/*!
This blueprint enables advanced staking of resources. Staking rewards are distributed periodically.

The 3 main advantages over simple OneResourcePool staking that are accomplished are:
- Staking reward can be a token different from the staked token.
- Staked tokens can be locked (e.g. for voting).
- An unstaking delay can be set (is technically also possible using the OneResourcePool).

To accomplish this, users now stake their tokens to a staking ID. The staked tokens are then held by the staking component:
- Rewards are claimed through the component, which can distribute any token as a reward.
- The component can easily lock these tokens.
- Unstaking is done by requesting an unstaking receipt, which can be redeemed through the component after a set delay, providing an unstaking delay.

This NFT staking ID approach has some disadvantages over simple OneResourcePool staking:
- Wallet display of staked tokens is more difficult, as staked amounts are stored by an NFT (staking ID). Ideally, users need to use some kind of front-end to see their staked tokens.
- Staking rewards are distributed periodically, not continuously.
- User needs to claim rewards manually. Though this could be automated in some way.
- Staked tokens are not liquid, making it impossible to use them in traditional DEXes. Though they are transferable to other user's staking IDs, so a DEX could be built on top of this system. This way, liquidity could be provided while still earning staking fees.
- It is more complex to set up and manage.
*/

use scrypto::prelude::*;

// NFT receipt structure, minted when an unstake is requested, redeemable after a set delay.
#[derive(ScryptoSbor, NonFungibleData)]
pub struct UnstakeReceipt {
    #[mutable]
    pub address: ResourceAddress,
    #[mutable]
    pub amount: Decimal,
    #[mutable]
    pub redemption_time: Instant,
}

// Staking ID structure, holding staked and locked amounts and date until which they are locked. Also stores the next period to claim rewards (updated after a user has claimed them).
#[derive(ScryptoSbor, NonFungibleData)]
pub struct Id {
    #[mutable]
    pub amounts_staked: Vec<Decimal>,
    #[mutable]
    pub amounts_locked: Vec<Decimal>,
    #[mutable]
    pub next_period: i64,
    #[mutable]
    pub locked_until: Vec<Option<Instant>>,
}

// Lock structure, holding the information about locking options of a token.
#[derive(ScryptoSbor)]
pub struct Lock {
    pub payment: Decimal,
    pub duration: i64,
}

// Stakable unit structure, used by the component to data about a stakable token.
#[derive(ScryptoSbor)]
pub struct StakableUnit {
    pub address: ResourceAddress,
    pub staked_amount: Decimal,
    pub vault: Vault,
    pub reward_amount: Decimal,
    pub lock: Lock,
    pub rewards: KeyValueStore<i64, Decimal>,
}

// Stake transfer receipt structure, minted when a user wants to transfer their staked tokens, redeemable by other users to add these tokens to their own staking ID.
#[derive(ScryptoSbor, NonFungibleData)]
pub struct StakeTransferReceipt {
    pub address: ResourceAddress,
    pub amount: Decimal,
}

#[blueprint]
mod staking {
    enable_method_auth! {
        methods {
            create_id => PUBLIC;
            stake => PUBLIC;
            start_unstake => PUBLIC;
            finish_unstake => PUBLIC;
            update_id => PUBLIC;
            update_period => PUBLIC;
            lock_stake => PUBLIC;
            set_lock => restrict_to: [OWNER];
            set_period_interval => restrict_to: [OWNER];
            set_rewards => restrict_to: [OWNER];
            set_max_claim_delay => restrict_to: [OWNER];
            fill_rewards => restrict_to: [OWNER];
            remove_rewards => restrict_to: [OWNER];
            add_stakable => restrict_to: [OWNER];
            edit_stakable => restrict_to: [OWNER];
            set_next_period_to_now => restrict_to: [OWNER];
            set_unstake_delay => restrict_to: [OWNER];
        }
    }

    struct Staking {
        // interval in which rewards are distributed in days
        period_interval: i64,
        // time the next interval starts
        next_period: Instant,
        // current period, starting at 0, incremented after each period_interval
        current_period: i64,
        // maximum amount of weeks rewards are stored for a user, after which they become unclaimable
        max_claim_delay: i64,
        // maximum unstaking delay the admin can set
        max_unstaking_delay: i64,
        // resource manager of the stake transfer receipts
        stake_transfer_receipt_manager: ResourceManager,
        // counter for the stake transfer receipts
        stake_transfer_receipt_counter: u64,
        // resource manager of the unstake receipts
        unstake_receipt_manager: ResourceManager,
        // counter for the unstake receipts
        unstake_receipt_counter: u64,
        // delay after which unstaked tokens can be redeemed in days
        unstake_delay: i64,
        // resource manager of the staking IDs
        id_manager: ResourceManager,
        // counter for the staking IDs
        id_counter: u64,
        // vault that stores staking rewards
        reward_vault: FungibleVault,
        // keyvaluestore, holding stakable units and their data
        stakes: KeyValueStore<ResourceAddress, StakableUnit>,
        // vector of stakable tokens
        stakables: Vec<ResourceAddress>,
        // whether a DAO is controlling the staking
        // If a centralized entity controls the controller badge, using the set_lock method, they could lock the someone's tokens by telling the system someone is voting.
        // To prevent this, this functionality only enabled if dao_controlled is set to true.
        dao_controlled: bool,
    }

    impl Staking {
        // this function instantiates the staking component
        //
        // ## INPUT
        // - `controller`: the address of the controller badge, which will be the owner of the staking component
        // - `rewards`: the initial rewards the staking component holds
        // - `period_interval`: the interval in which rewards are distributed in days
        // - `name`: the name of your project
        // - `symbol`: the symbol of your project
        //
        // ## OUTPUT
        // - the staking component
        //
        // ## LOGIC
        // - all resource managers are created
        // - the rewards are put into the reward vault and other values are set appropriately
        // - the staking component is instantiated
        pub fn new(
            controller: ResourceAddress,
            rewards: FungibleBucket,
            period_interval: i64,
            name: String,
            symbol: String,
            dao_controlled: bool,
            max_unstaking_delay: i64,
        ) -> Global<Staking> {
            let (address_reservation, component_address) =
                Runtime::allocate_component_address(Staking::blueprint_id());

            let id_manager = ResourceBuilder::new_integer_non_fungible::<Id>(OwnerRole::Fixed(
                rule!(require(controller)),
            ))
            .metadata(metadata!(
                init {
                    "name" => format!("{} Staking ID", name), updatable;
                    "symbol" => format!("id{}", symbol), updatable;
                    "description" => format!("An ID recording your stake in the {} ecosystem.", name), updatable;
                }
            ))
            .mint_roles(mint_roles!(
                minter => rule!(require(global_caller(component_address))
                || require_amount(
                    dec!("0.75"),
                    controller
                ));
                minter_updater => rule!(deny_all);
            ))
            .burn_roles(burn_roles!(
                burner => rule!(deny_all);
                burner_updater => rule!(deny_all);
            ))
            .withdraw_roles(withdraw_roles!(
                withdrawer => rule!(deny_all);
                withdrawer_updater => rule!(deny_all);
            ))
            .non_fungible_data_update_roles(non_fungible_data_update_roles!(
                non_fungible_data_updater => rule!(require(global_caller(component_address))
                || require_amount(
                    dec!("0.75"),
                    controller
                ));
                non_fungible_data_updater_updater => rule!(deny_all);
            ))
            .create_with_no_initial_supply();

            let stake_transfer_receipt_manager = ResourceBuilder::new_integer_non_fungible::<UnstakeReceipt>(
                OwnerRole::Fixed(rule!(require(controller))),
            )
            .metadata(metadata!(
                init {
                    "name" => format!("{} Stake Transfer Receipt", name), updatable;
                    "symbol" => format!("staketr{}", symbol), updatable;
                    "description" => format!("An stake transfer receipt used in the {} ecosystem.", name), updatable;
                }
            ))            
            .mint_roles(mint_roles!(
                minter => rule!(require(global_caller(component_address)));
                minter_updater => rule!(deny_all);
            ))
            .burn_roles(burn_roles!(
                burner => rule!(require(global_caller(component_address)));
                burner_updater => rule!(deny_all);
            ))
            .create_with_no_initial_supply();

            let unstake_receipt_manager =
                ResourceBuilder::new_integer_non_fungible::<UnstakeReceipt>(OwnerRole::Fixed(
                    rule!(require(controller)),
                ))
                .metadata(metadata!(
                    init {
                        "name" => format!("{} Unstake Receipt", name), updatable;
                        "symbol" => format!("unstake{}", symbol), updatable;
                        "description" => format!("An unstake receipt used in the {} ecosystem.", name), updatable;
                    }
                ))   
                .mint_roles(mint_roles!(
                    minter => rule!(require(global_caller(component_address)));
                    minter_updater => rule!(deny_all);
                ))
                .burn_roles(burn_roles!(
                    burner => rule!(require(global_caller(component_address)));
                    burner_updater => rule!(deny_all);
                ))
                .non_fungible_data_update_roles(non_fungible_data_update_roles!(
                    non_fungible_data_updater => rule!(require(global_caller(component_address)));
                    non_fungible_data_updater_updater => rule!(deny_all);
                ))
                .create_with_no_initial_supply();

            Self {
                next_period: Clock::current_time_rounded_to_minutes()
                    .add_days(period_interval)
                    .unwrap(),
                period_interval,
                current_period: 0,
                max_claim_delay: 5,
                max_unstaking_delay,
                unstake_delay: 7,
                id_manager,
                stake_transfer_receipt_manager,
                stake_transfer_receipt_counter: 0,
                unstake_receipt_manager,
                unstake_receipt_counter: 0,
                id_counter: 0,
                reward_vault: FungibleVault::with_bucket(rewards.as_fungible()),
                stakes: KeyValueStore::new(),
                stakables: vec![],
                dao_controlled,
            }
            .instantiate()
            .prepare_to_globalize(OwnerRole::Fixed(rule!(require(controller))))
            .with_address(address_reservation)
            .globalize()
        }

        // this method updates the component's period and saves the rewards accompanying the period
        //
        // ## INPUT
        // - none
        //
        // ## OUTPUT
        // - none
        // 
        // ## LOGIC
        // - the method calculates the number of extra periods that have passed since the last update, because the method might not be called exactly at the end of a period
        // - if a period has passed, for each stakable token the rewards are calculated and recorded, reward calculation is relatively simple:
        //    - every stakable has a total amount of reward per period
        //    - total reward amount is divided by the total amount staked to get the reward per staked token
        // - the current period is incremented and the next period is set
        pub fn update_period(&mut self) {
            let extra_periods_dec: Decimal = ((Clock::current_time_rounded_to_minutes()
                .seconds_since_unix_epoch
                - self.next_period.seconds_since_unix_epoch)
                / (Decimal::from(self.period_interval) * dec!(86400)))
            .checked_floor()
            .unwrap();

            let extra_periods: i64 = i64::try_from(extra_periods_dec.0 / Decimal::ONE.0).unwrap();

            if Clock::current_time_is_at_or_after(self.next_period, TimePrecision::Minute) {
                for stakable in self.stakables.iter() {
                    let stakable_unit = self.stakes.get_mut(stakable).unwrap();
                    if stakable_unit.staked_amount > dec!(0) {
                        stakable_unit.rewards.insert(
                            self.current_period,
                            stakable_unit.reward_amount / stakable_unit.staked_amount,
                        );
                    } else {
                        stakable_unit.rewards.insert(self.current_period, dec!(0));
                    }
                }

                self.current_period += 1;
                self.next_period = self
                    .next_period
                    .add_days((1 + extra_periods) * self.period_interval)
                    .unwrap();
            }
        }
        // This method requests an unstake of staked tokens
        //
        // ## INPUT
        // - `id_proof`: the proof of the staking ID
        // - `address`: the address of the stakable token
        // - `unstake_amount`: the amount of tokens to unstake
        // - `unstake_all`: whether to unstake all tokens (useful to ensure no dust is leftover)
        // - `stake_transfer`: whether to transfer the staked tokens to another user
        //
        // ## OUTPUT
        // - the unstake receipt / transfer receipt
        //
        // ## LOGIC
        // - the method checks the staking ID
        // - the method checks the staked amount
        // - the method checks if the staked tokens are locked (then unstaking is not possible)
        // - if not tokens are removed from staking ID stake
        // - if the user wants to transfer the tokens, a transfer receipt is minted
        // - if the user wants to unstake the tokens, an unstake receipt is minted
        pub fn start_unstake(
            &mut self,
            id_proof: NonFungibleProof,
            address: ResourceAddress,
            unstake_amount: Decimal,
            unstake_all: bool,
            stake_transfer: bool,
        ) -> Bucket {
            let id_proof =
                id_proof.check_with_message(self.id_manager.address(), "Invalid Id supplied!");

            let id = id_proof.non_fungible::<Id>().local_id().clone();

            self.check_indexes(&id);

            let id_data: Id = self.id_manager.get_non_fungible_data(&id);
            let index = self.stakables.iter().position(|&r| r == address).unwrap();
            let mut staked_vector: Vec<Decimal> = id_data.amounts_staked.clone();
            let locked_vector: Vec<Option<Instant>> = id_data.locked_until.clone();

            assert!(
                staked_vector[index] > dec!(0),
                "No stake available to unstake."
            );

            if locked_vector[index].is_some() {
                assert!(
                    Clock::current_time_is_at_or_after(
                        locked_vector[index].unwrap(),
                        TimePrecision::Minute
                    ),
                    "You cannot unstake tokens currently participating in a vote."
                );
            }

            let mut amount: Decimal = match unstake_all {
                true => staked_vector[index],
                false => unstake_amount,
            };

            if amount >= staked_vector[index] {
                self.stakes.get_mut(&address).unwrap().staked_amount -= staked_vector[index];
                amount = staked_vector[index];
                staked_vector[index] = dec!(0);
            } else {
                self.stakes.get_mut(&address).unwrap().staked_amount -= amount;
                staked_vector[index] -= amount;
            }

            self.id_manager
                .update_non_fungible_data(&id, "amounts_staked", staked_vector);

            if stake_transfer {
                let stake_transfer_receipt = StakeTransferReceipt {
                    address,
                    amount,
                };
                self.stake_transfer_receipt_counter += 1;
                self.stake_transfer_receipt_manager.mint_non_fungible(
                    &NonFungibleLocalId::integer(self.stake_transfer_receipt_counter),
                    stake_transfer_receipt,
                )
            } else {
                let unstake_receipt = UnstakeReceipt {
                    address,
                    amount,
                    redemption_time: Clock::current_time_rounded_to_minutes()
                        .add_days(self.unstake_delay)
                        .unwrap(),
                };
                self.unstake_receipt_counter += 1;
                self.unstake_receipt_manager.mint_non_fungible(
                    &NonFungibleLocalId::integer(self.unstake_receipt_counter),
                    unstake_receipt,
                )
            }
        }

        // This method finishes an unstake, redeeming the unstaked tokens
        //
        // ## INPUT
        // - `receipt`: the unstake receipt
        //
        // ## OUTPUT
        // - the unstaked tokens
        //
        // ## LOGIC
        // - the method checks the receipt
        // - the method checks the redemption time
        // - the method burns the receipt
        // - the method returns the unstaked tokens
        pub fn finish_unstake(&mut self, receipt: Bucket) -> Bucket {
            assert!(receipt.resource_address() == self.unstake_receipt_manager.address());

            let receipt_data = receipt
                .as_non_fungible()
                .non_fungible::<UnstakeReceipt>()
                .data();

            assert!(
                Clock::current_time_is_at_or_after(
                    receipt_data.redemption_time,
                    TimePrecision::Minute
                ),
                "You cannot unstake tokens before the redemption time."
            );

            receipt.burn();

            self.stakes
                .get_mut(&receipt_data.address)
                .unwrap()
                .vault
                .take(receipt_data.amount)
        }

        // This method creates a new staking ID
        //
        // ## INPUT
        // - none
        //
        // ## OUTPUT
        // - the staking ID
        //
        // ## LOGIC
        // - the method increments the ID counter
        // - the method creates a new ID
        // - the method returns the ID
        pub fn create_id(&mut self) -> Bucket {
            self.id_counter += 1;

            let id_data = Id {
                amounts_staked: vec![dec!(0); self.stakables.len()],
                amounts_locked: vec![dec!(0); self.stakables.len()],
                next_period: self.current_period + 1,
                locked_until: vec![None; self.stakables.len()],
            };

            let id: Bucket = self
                .id_manager
                .mint_non_fungible(&NonFungibleLocalId::integer(self.id_counter), id_data);

            id
        }

        // This method stakes tokens to a staking ID
        //
        // ## INPUT
        // - `address`: the address of the stakable token
        // - `stake_bucket`: an optional bucket of the staked tokens
        // - `id_proof`: the proof of the staking ID
        // - `stake_transfer_receipt`: an optional stake transfer receipt
        //
        // ## OUTPUT
        // - none
        //
        // ## LOGIC
        // - the method checks the staking ID
        // - the method checks if latest rewards have been claimed, if not, the method fails
        // - the method checks the to be staked tokens, adds it to the to be staked amount, adds tokens to stake vault
        // - the method checks the to be staked transfer receipt, adds it to the to be staked amount, burns transfer receipt
        // - the method updates the staking ID
        pub fn stake(&mut self, address: ResourceAddress, stake_bucket: Option<Bucket>, id_proof: NonFungibleProof, stake_transfer_receipt: Option<NonFungibleBucket>) {
            let id_proof =
                id_proof.check_with_message(self.id_manager.address(), "Invalid Id supplied!");
            let id = id_proof.non_fungible::<Id>().local_id().clone();
            self.check_indexes(&id);
            let id_data: Id = self.id_manager.get_non_fungible_data(&id);
            let index = self.stakables.iter().position(|&r| r == address).unwrap();
            let mut staked_vector: Vec<Decimal> = id_data.amounts_staked.clone();
            let mut stake_amount: Decimal = dec!(0);
            assert!(
                id_data.next_period >= self.current_period,
                "Please claim unclaimed rewards on your ID before staking."
            );
            assert!(self.stakables.contains(&address), "This requested token is not stakable.");

            if let Some(bucket) = stake_bucket {
                assert!(bucket.resource_address() == address, "Token supplied does not match requested stakable token.");
                stake_amount += bucket.amount();
                self.stakes
                    .get_mut(&address)
                    .unwrap()
                    .vault
                    .put(bucket);
            }

            if let Some(receipt) = stake_transfer_receipt {
                assert!(receipt.resource_address() == self.stake_transfer_receipt_manager.address(), "Wrong stake transfer receipt supplied.");
                let receipt_data = receipt.non_fungible::<StakeTransferReceipt>().data();
                assert!(receipt_data.address == address, "Token found in stake transfer receipt does not match requested stakable token.");
                stake_amount += receipt_data.amount;
                receipt.burn();
            }

            staked_vector[index] += stake_amount;

            self.id_manager
                .update_non_fungible_data(&id, "amounts_staked", staked_vector);

            self.stakes.get_mut(&address).unwrap().staked_amount += stake_amount;

            self.id_manager.update_non_fungible_data(
                &id,
                "next_period",
                self.current_period + 1,
            );
        }

        // This method claims rewards from a staking ID
        //
        // ## INPUT
        // - `id_proof`: the proof of the staking ID
        //
        // ## OUTPUT
        // - the claimed rewards
        //
        // ## LOGIC
        // - the method checks the staking ID
        // - the method checks amount of unclaimed periods
        // - the method iterates over all staked tokens and calculates the rewards
        // - the method updates the staking ID to the next period
        // - the method returns the claimed rewards
        pub fn update_id(&mut self, id_proof: NonFungibleProof) -> FungibleBucket {
            let id_proof =
                id_proof.check_with_message(self.id_manager.address(), "Invalid Id supplied!");
            let id = id_proof.non_fungible::<Id>().local_id().clone();
            self.check_indexes(&id);

            let id_data: Id = self.id_manager.get_non_fungible_data(&id);
            let staked_vector: Vec<Decimal> = id_data.amounts_staked.clone();

            let mut claimed_weeks: i64 = self.current_period - id_data.next_period + 1;
            if claimed_weeks > self.max_claim_delay {
                claimed_weeks = self.max_claim_delay;
            }

            assert!(claimed_weeks > 0, "Wait longer to claim your rewards.");

            let mut staking_reward: Decimal = dec!(0);

            self.id_manager
                .update_non_fungible_data(&id, "next_period", self.current_period + 1);

            for (index, stakable) in self.stakables.iter().enumerate() {
                let stakable_unit = self.stakes.get_mut(stakable).unwrap();
                for week in 1..(claimed_weeks + 1) {
                    if stakable_unit
                        .rewards
                        .get(&(self.current_period - week))
                        .is_some()
                    {
                        staking_reward += *stakable_unit
                            .rewards
                            .get(&(self.current_period - week))
                            .unwrap()
                            * staked_vector[index]
                    }
                }
            }

            self.reward_vault.take(staking_reward)
        }

        // This method locks staked tokens for a certain duration and gives rewards for locking them
        //
        // ## INPUT
        // - `address`: the address of the stakable token
        // - `id_proof`: the proof of the staking ID
        //
        // ## OUTPUT
        // - rewards for locking the tokens
        //
        // ## LOGIC
        // - the method checks the staking ID
        // - the method checks the stakables for a matching address
        // - the method checks whether the staking ID tokens are already locked
        // - the method locks the tokens by updating the staking ID
        // - the method returns the rewards for locking the tokens


        pub fn lock_stake(&mut self, address: ResourceAddress, id_proof: NonFungibleProof) -> Bucket {
            let id_proof =
                id_proof.check_with_message(self.id_manager.address(), "Invalid Id supplied!");
            let id = id_proof.non_fungible::<Id>().local_id().clone();

            self.check_indexes(&id);
            let index = self.stakables.iter().position(|&r| r == address).expect("Stakable not found.");
            let stakable = self.stakes.get(&address).unwrap();

            let id_data: Id = self.id_manager.get_non_fungible_data(&id);
            let staked_amount: Decimal = id_data.amounts_staked[index];        
            let mut locked_vector: Vec<Option<Instant>> = id_data.locked_until.clone();          
            if locked_vector[index].is_some() {
                assert!(Clock::current_time_is_at_or_after(locked_vector[index].unwrap(), TimePrecision::Minute), "Tokens are already locked.");
            }

            let lock_until: Instant = Clock::current_time_rounded_to_minutes().add_days(stakable.lock.duration).unwrap();      
            locked_vector[index] = Some(lock_until);

            self.id_manager
                .update_non_fungible_data(&id, "locked_until", locked_vector);

            self.reward_vault.take(stakable.lock.payment * staked_amount).into()
        }

        //////////////////////////////////////////////////////////////////////
        ////////////////////////////ADMIN METHODS/////////////////////////////
        //////////////////////////////////////////////////////////////////////

        pub fn set_period_interval(&mut self, new_interval: i64) {
            self.period_interval = new_interval;
        }

        pub fn fill_rewards(&mut self, bucket: Bucket) {
            self.reward_vault.put(bucket.as_fungible());
        }

        pub fn remove_rewards(&mut self, amount: Decimal) -> Bucket {
            self.reward_vault.take(amount).into()
        }

        pub fn set_max_claim_delay(&mut self, new_delay: i64) {
            self.max_claim_delay = new_delay;
        }

        pub fn set_unstake_delay(&mut self, new_delay: i64) {
            assert!(new_delay <= self.max_unstaking_delay, "Unstaking delay cannot be longer than the maximum unstaking delay.");
            self.unstake_delay = new_delay;
        }

        pub fn set_rewards(&mut self, address: ResourceAddress, reward: Decimal) {
            self.stakes.get_mut(&address).unwrap().reward_amount = reward;
        }

        pub fn add_stakable(&mut self, address: ResourceAddress, reward_amount: Decimal, lock: Lock) {
            self.stakes.insert(
                address,
                StakableUnit {
                    address,
                    staked_amount: dec!(0),
                    vault: Vault::new(address),
                    reward_amount,
                    lock,
                    rewards: KeyValueStore::new(),
                },
            );

            self.stakables.push(address);
        }

        pub fn edit_stakable(&mut self, address: ResourceAddress, reward_amount: Decimal, lock: Lock) {
            let mut stakable = self.stakes.get_mut(&address).unwrap();
            stakable.reward_amount = reward_amount;
            stakable.lock = lock;
        }

        pub fn set_next_period_to_now(&mut self) {
            self.next_period = Clock::current_time_rounded_to_minutes();
        }

        // This method locks staked tokens for voting
        //
        // ## INPUT
        // - `address`: the address of the stakable token
        // - `lock_until`: the date until which the tokens are locked
        // - `id`: the staking ID
        //
        // ## OUTPUT
        // - none
        //
        // ## LOGIC
        // - the method checks the staking ID
        // - the method updates the locked_until field of the staking ID appropriately
        
        pub fn set_lock(&mut self, address: ResourceAddress, lock_until: Instant, id: NonFungibleLocalId) {
            assert!(self.dao_controlled == true, "This functionality is only available if a DAO is controlling the staking.");
            let id_data: Id = self.id_manager.get_non_fungible_data(&id);
            let index = self.stakables.iter().position(|&r| r == address).unwrap();
            let mut locked_vector: Vec<Option<Instant>> = id_data.locked_until.clone();
            locked_vector[index] = Some(lock_until);

            self.id_manager
                .update_non_fungible_data(&id, "locked_until", locked_vector);
        }

        //////////////////////////////////////////////////////////////////////
        ////////////////////////////HELPER METHODS////////////////////////////
        //////////////////////////////////////////////////////////////////////

        // This method checks the indexes of the staking ID, adding new indexes if necessary. Useful if new stakables are added since the staking ID was created / last used.
        //
        // ## INPUT
        // - `id`: the staking ID
        //
        // ## OUTPUT
        // - none
        //
        // ## LOGIC
        // - the method updates the period if necessary, so the next period and rewwards are always up to date
        // - the method checks the staking ID
        // - the method checks the stakables
        // - the method adds new indexes if necessary

        fn check_indexes(&mut self, id: &NonFungibleLocalId) {
            if Clock::current_time_is_at_or_after(self.next_period, TimePrecision::Minute) {
                self.update_period();
            }
            let id_data: Id = self.id_manager.get_non_fungible_data(id);
            let mut staked_vector: Vec<Decimal> = id_data.amounts_staked.clone();
            let mut locked_vector: Vec<Option<Instant>> = id_data.locked_until.clone();

            if staked_vector.len() != self.stakables.len() {
                let to_add_items = self.stakables.len() - staked_vector.len();
                let to_add_vector = vec![dec!(0); to_add_items];
                let to_add_locked_vector: Vec<Option<Instant>> = vec![None; to_add_items];
                staked_vector.extend(to_add_vector.clone());
                locked_vector.extend(to_add_locked_vector.clone());

                self.id_manager
                    .update_non_fungible_data(id, "amounts_staked", staked_vector);

                self.id_manager
                    .update_non_fungible_data(id, "locked_until", locked_vector);
            }
        }
    }
}
