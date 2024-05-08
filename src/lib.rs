use scrypto::prelude::*;

#[derive(ScryptoSbor, NonFungibleData)]
pub struct UnstakeReceipt {
    #[mutable]
    pub address: ResourceAddress,
    #[mutable]
    pub amount: Decimal,
    #[mutable]
    pub redemption_time: Instant,
}

#[derive(ScryptoSbor, NonFungibleData)]
pub struct Id {
    #[mutable]
    pub amounts_staked: Vec<Decimal>,
    #[mutable]
    pub next_period: i64,
    #[mutable]
    pub voting_until: Vec<Option<Instant>>,
}

#[derive(ScryptoSbor)]
pub struct StakableUnit {
    pub address: ResourceAddress,
    pub staked_amount: Decimal,
    pub vault: Vault,
    pub reward_amount: Decimal,
    pub rewards: KeyValueStore<i64, Decimal>,
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
            set_period_interval => restrict_to: [OWNER];
            set_rewards => restrict_to: [OWNER];
            set_max_claim_delay => restrict_to: [OWNER];
            update_period => restrict_to: [OWNER];
            fill_rewards => restrict_to: [OWNER];
            remove_rewards => restrict_to: [OWNER];
            add_stakable => restrict_to: [OWNER];
            set_next_period_to_now => restrict_to: [OWNER];
            set_unstake_delay => restrict_to: [OWNER];
        }
    }

    struct Staking {
        period_interval: i64,
        next_period: Instant,
        current_period: i64,
        max_claim_delay: i64,
        unstake_receipt_manager: ResourceManager,
        unstake_receipt_counter: u64,
        unstake_delay: i64,
        id_manager: ResourceManager,
        id_counter: u64,
        reward_vault: FungibleVault,
        stakes: KeyValueStore<ResourceAddress, StakableUnit>,
        stakables: Vec<ResourceAddress>,
    }

    impl Staking {
        pub fn new(
            controller: ResourceAddress,
            rewards: FungibleBucket,
            period_interval: i64,
        ) -> (Global<Staking>, ResourceManager, ResourceManager) {
            let (address_reservation, component_address) =
                Runtime::allocate_component_address(Staking::blueprint_id());

            let id_manager = ResourceBuilder::new_integer_non_fungible::<Id>(OwnerRole::Fixed(
                rule!(require(controller)),
            ))
            .metadata(metadata!(
                init {
                    "name" => "Staking ID", updatable;
                    "symbol" => "stakeID", updatable;
                    "description" => "An ID recording your stake", updatable;
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
                    controller));
                non_fungible_data_updater_updater => rule!(deny_all);
            ))
            .create_with_no_initial_supply();

            let unstake_receipt_manager =
                ResourceBuilder::new_integer_non_fungible::<UnstakeReceipt>(OwnerRole::Fixed(
                    rule!(require(controller)),
                ))
                .metadata(metadata!(
                    init {
                        "name" => "Unstaking Receipt", updatable;
                        "symbol" => "UNSTAKE", updatable;
                        "description" => "A receipt used to unstake a resource", updatable;
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
                    burner => rule!(require(global_caller(component_address))
                    || require_amount(
                        dec!("0.75"),
                        controller
                    ));
                    burner_updater => rule!(deny_all);
                ))
                .non_fungible_data_update_roles(non_fungible_data_update_roles!(
                    non_fungible_data_updater => rule!(require(global_caller(component_address))
                    || require_amount(
                        dec!("0.75"),
                        controller));
                    non_fungible_data_updater_updater => rule!(deny_all);
                ))
                .create_with_no_initial_supply();

            let staking = Self {
                next_period: Clock::current_time_rounded_to_minutes()
                    .add_days(period_interval)
                    .unwrap(),
                period_interval,
                current_period: 0,
                max_claim_delay: 5,
                unstake_delay: 7,
                id_manager,
                unstake_receipt_manager,
                unstake_receipt_counter: 0,
                id_counter: 0,
                reward_vault: FungibleVault::with_bucket(rewards.as_fungible()),
                stakes: KeyValueStore::new(),
                stakables: vec![],
            }
            .instantiate()
            .prepare_to_globalize(OwnerRole::Fixed(rule!(require(controller))))
            .with_address(address_reservation)
            .globalize();

            (staking, id_manager, unstake_receipt_manager)
        }

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

        pub fn start_unstake(
            &mut self,
            id_proof: NonFungibleProof,
            address: ResourceAddress,
            unstake_amount: Decimal,
            unstake_all: bool,
        ) -> Bucket {
            let id_proof =
                id_proof.check_with_message(self.id_manager.address(), "Invalid Id supplied!");

            let id = id_proof.non_fungible::<Id>().local_id().clone();

            self.check_indexes(id.clone());

            let id_data: Id = self.id_manager.get_non_fungible_data(&id);
            let index = self.stakables.iter().position(|&r| r == address).unwrap();
            let mut staked_vector: Vec<Decimal> = id_data.amounts_staked.clone();
            let voting_vector: Vec<Option<Instant>> = id_data.voting_until.clone();

            assert!(
                staked_vector[index] > dec!(0),
                "No stake available to unstake."
            );

            if voting_vector[index].is_some() {
                assert!(
                    Clock::current_time_is_at_or_after(
                        voting_vector[index].unwrap(),
                        TimePrecision::Minute
                    ),
                    "You cannot unstake tokens currently participating in a vote."
                );
            }

            let amount: Decimal = match unstake_all {
                true => staked_vector[index],
                false => unstake_amount,
            };

            if amount >= staked_vector[index] {
                self.stakes.get_mut(&address).unwrap().staked_amount -= staked_vector[index];
                staked_vector[index] = dec!(0);
            } else {
                self.stakes.get_mut(&address).unwrap().staked_amount -= amount;
                staked_vector[index] -= amount;
            }

            self.id_manager
                .update_non_fungible_data(&id, "amounts_staked", staked_vector);

            let unstake_receipt = UnstakeReceipt {
                address,
                amount,
                redemption_time: Clock::current_time_rounded_to_minutes()
                    .add_days(self.unstake_delay)
                    .unwrap(),
            };

            let receipt: Bucket = self.unstake_receipt_manager.mint_non_fungible(
                &NonFungibleLocalId::integer(self.unstake_receipt_counter),
                unstake_receipt,
            );

            receipt
        }

        pub fn finish_unstake(&mut self, receipt: Bucket) -> Bucket {
            assert!(receipt.resource_address() == self.unstake_receipt_manager.address());

            let receipt_id: NonFungibleLocalId = receipt.as_non_fungible().non_fungible_local_id();

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

            self.unstake_receipt_manager
                .update_non_fungible_data(&receipt_id, "amount", dec!(0));

            self.stakes
                .get_mut(&receipt_data.address)
                .unwrap()
                .vault
                .take(receipt_data.amount)
        }

        pub fn create_id(&mut self) -> Bucket {
            self.id_counter += 1;

            let id_data = Id {
                amounts_staked: vec![dec!(0); self.stakables.len()],
                next_period: self.current_period + 1,
                voting_until: vec![None; self.stakables.len()],
            };

            let id: Bucket = self
                .id_manager
                .mint_non_fungible(&NonFungibleLocalId::integer(self.id_counter), id_data);

            id
        }

        pub fn stake(&mut self, stake_bucket: Bucket, id_proof: NonFungibleProof) {
            let id_proof =
                id_proof.check_with_message(self.id_manager.address(), "Invalid Id supplied!");
            let id = id_proof.non_fungible::<Id>().local_id().clone();
            self.check_indexes(id.clone());

            let address: ResourceAddress = stake_bucket.resource_address();

            assert!(self.stakables.contains(&address), "Wrong token supplied");

            let id_data: Id = self.id_manager.get_non_fungible_data(&id);
            let index = self.stakables.iter().position(|&r| r == address).unwrap();
            let mut staked_vector: Vec<Decimal> = id_data.amounts_staked.clone();

            assert!(
                id_data.next_period >= self.current_period,
                "Please claim unclaimed rewards on your ID before staking."
            );

            staked_vector[index] += stake_bucket.amount();

            self.id_manager
                .update_non_fungible_data(&id, "amounts_staked", staked_vector);

            self.stakes.get_mut(&address).unwrap().staked_amount += stake_bucket.amount();

            self.id_manager
                .update_non_fungible_data(&id, "next_period", self.current_period + 1);
            self.stakes
                .get_mut(&address)
                .unwrap()
                .vault
                .put(stake_bucket);
        }

        pub fn update_id(&mut self, id_proof: NonFungibleProof) -> Bucket {
            let id_proof =
                id_proof.check_with_message(self.id_manager.address(), "Invalid Id supplied!");
            let id = id_proof.non_fungible::<Id>().local_id().clone();
            self.check_indexes(id.clone());

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

            self.reward_vault.take(staking_reward).into()
        }

        ////////////////////////////ADMIN METHODS////////////////////////////

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
            self.unstake_delay = new_delay;
        }

        pub fn set_rewards(&mut self, address: ResourceAddress, reward: Decimal) {
            self.stakes.get_mut(&address).unwrap().reward_amount = reward;
        }

        pub fn add_stakable(&mut self, address: ResourceAddress, reward_amount: Decimal) {
            self.stakes.insert(
                address,
                StakableUnit {
                    address,
                    staked_amount: dec!(0),
                    vault: Vault::new(address),
                    reward_amount,
                    rewards: KeyValueStore::new(),
                },
            );

            self.stakables.push(address);
        }

        pub fn set_next_period_to_now(&mut self) {
            self.next_period = Clock::current_time_rounded_to_minutes();
        }

        ////////////////////////////HELPER METHODS////////////////////////////

        fn check_indexes(&self, id: NonFungibleLocalId) {
            let id_data: Id = self.id_manager.get_non_fungible_data(&id);
            let mut staked_vector: Vec<Decimal> = id_data.amounts_staked.clone();
            let mut voting_vector: Vec<Option<Instant>> = id_data.voting_until.clone();

            if staked_vector.len() != self.stakables.len() {
                let to_add_items = self.stakables.len() - staked_vector.len();
                let to_add_vector = vec![dec!(0); to_add_items];
                let to_add_voting_vector: Vec<Option<Instant>> = vec![None; to_add_items];
                staked_vector.extend(to_add_vector.clone());
                voting_vector.extend(to_add_voting_vector.clone());

                self.id_manager
                    .update_non_fungible_data(&id, "amounts_staked", staked_vector);

                self.id_manager
                    .update_non_fungible_data(&id, "voting_until", voting_vector);
            }
        }
    }
}
