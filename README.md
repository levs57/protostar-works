# Circuits in generic arithmetization / protostar implementation

In this library we are implementing Protostar folding scheme https://eprint.iacr.org/2023/620 using generic arithmetization and researching efficiency of various new tricks possible in it.

## Setting up

Clone the repo, run
``` rustup override set nightly ```
then build it.

## Benchmarks

Main prover work in protostar consists of two components - MSM of witness size, and computation of the cross-terms (which requires to run d times every constraint of degree d). As currently execution of gates is not parallelized (and we just want to get sense of the relative costs of these two operations) we are benching everything in a single thread.

Tests were done on a computer with the following specs:
#### Intel(R) Core(TM) i7-7700 CPU @ 3.60GHz   3.60 GHz

Parallelization will slightly skew it towards MSM being heavier (because Pippinger algorithm does not parallelize perfectly, and execution of constraints on random vectors does), but in practice we will likely have circuits of reasonably large size, which will offset this.

-----

|                    | escalarmul (128 bit) |
| ------------------ | -------------------- |
| witness size       | 523                  |
| msm time           | 12.548 ms            |
| cross-terms time   | 3.2342 ms            |
| r1cs witness size  | ~1300 ?              |

We do not have readily available quadratic circuit, so we have taken the witness size estimate for it from the cyclefold paper, but our back-of-the-envelope estimates agree with this evaluation.

One can also consider circuits with even higher degree gates for the Grumpkin curve (which will drastically improve the decider even further).

----

|                    | Poseidon(1 argument) |
| ------------------ | -------------------- |
| witness size       | 66                   |
| msm time           | 0.74878 msm (adjusted from 1000x-sized circuit)           |
| cross-terms time   | 0.25925 msm (adjusted from 1000x-sized circuit)|
| r1cs witness size  | 211                  |



Witness sizes for Poseidons of different arity compared to R1CS witness sizes:

| arity | our circuit | r1cs |
| ----- | ------------| -----| 
| 1     | 68          | 213  |
| 2     | 75          | 243  |
| 3     | 80          | 265  |
| 4     | 90          | 302  |
| 5     | 96          | 327  |
| 6     | 105         | 361  |
| 7     | 112         | 389  |
| 8     | 117         | 411  |
| 9     | 120         | 427  |
| 10    | 132         | 470  |


Here, one can see that our advantage in witness size even increases slightly with arity (from 3.1 to roughly 3.6); though the circuit involves the gate of relatively large degree 25. Provided no other useful gates of similar degree are found, this will likely be not that viable because of the recursion overhead.

## Using the library

Currently, the API is unstable and leaky, so use this at your own risk! If you want to try, check out the test.rs file and gadgets.

### Quick guide:

1. Before creating the circuit, you need to decide on sources of public values. Challenges are not different from public inputs, they are just public inputs given after the first round. These are created using ```ExternalValue```, and can be shared between interacting parties (so you can emulate interactive protocols or have some parallel proving strategy).

2. Variables can be created using ```circuit.advice(...)``` (from other variables and external values), and constrained with arbitrary black-box polynomials using ```circuit.constrain(...)```. It will try to check that the black-box function user provided is, indeed, a polynomial of claimed degree, at least in random points.It is theoretically possible to use the description of polynomial involving division, provided you guarantee that it will not fail on your inputs. You can also use ```circuit.apply(...)``` to apply polynomial operation and constrain the result in one command. You can also write gadgets abstracting complex functionalities - check what's already done in /src/gadgets/

3. After building the circuit, call ```circuit.finalize();```, now we are in the execution phase. You can call ```circuit.execute(r)```, to progress execution up to the round ```r```. Also make sure to provide it with external values for the corresponding round.

4. Check that all your constraints are satisfied using ```circuit.cs.validate_witness();```.
