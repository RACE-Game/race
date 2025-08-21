---
description: >-
  This section further researches specific steps, benefits and challenges for
  the brainstormed ideas
---

# Verifiable Randomness Generation

### To do list:

1. **Design the Zero-Knowledge Proof System**:
   1. **Define the statement to be proven**: In this case, the statement is that the <mark style="background-color:yellow;">Transactor & Validator servers have correctly shuffled the encrypted identifiers of the possible outcomes according to the mental poker algorithm</mark>.
   2. **Choose a suitable zero-knowledge proof scheme**: 01js offers various proof schemes, such as Groth16 and Plonk. The choice will depend on factors like proof size, verification time, and the complexity of the statement to be proven.
   3. **Develop the prover and verifier algorithms**: These algorithms will be implemented in the Transactor server and client applications, respectively. The prover generates the zero-knowledge proof, and the verifier checks its validity.
2. **Modify the Servers:**
   1. **Integrate 01js library**: Add the 01js library to the Transactor/Validator server's dependencies.
   2. **Generate the proof during shuffling**: After the Transactor and Validators shuffle the encrypted identifiers, they use 01js to generate a zero-knowledge proof of the correct shuffling.
   3. **Broadcast the proof**: Along with the shuffled identifiers, the servers broadcast the zero-knowledge proofs to all connected clients and validator servers.
3. **Modify the Client Applications**:
   1. **Integrate 01js library**: Add the 01js library to the Race SDK or directly into the client application code.
   2. **Verify the proof upon receiving**: When the client receives the shuffled identifiers and the proof from the servers, it uses 01js to verify the proof's validity. This ensures that the shuffling was done correctly without revealing the secret keys or the order of the outcomes.
4. **Adapt the Game Handler (Optional)**:
   1. If necessary, modify the game handler within the WASM bundle to handle the zero-knowledge proof verification. This may involve exposing a function that allows the client application to pass the proof to the game handler for additional verification.

### Benefits:

* **Enhanced Verifiability**: Players can independently verify the correctness of the shuffling process, increasing trust and transparency in the game.
* **Reduced Trust in the Transactor**: Players do not need to blindly trust the Transactor server, as they can verify the integrity of the randomness generation through the zero-knowledge proof. Though same is achieved in the existing implementation, Zero-Knowledge would provide an extra level of verification.
* **Improved Security**: The use of zero-knowledge proofs adds an extra layer of security to the randomization process, making it more difficult for malicious actors to manipulate the outcome.

### Challenges:

* **Performance Overhead**: Generating and verifying zero-knowledge proofs can be computationally expensive, especially on resource-constrained devices like mobile phones. Careful optimization and selection of proof schemes will be necessary to ensure acceptable performance.
* **Increased Complexity**: Adding zero-knowledge proofs to the protocol increases the overall complexity of the system. Developers need to carefully design and implement the proof system and integrate it seamlessly into the existing architecture.

### Conclusion:

Despite the challenges, integrating 01js and zero-knowledge proofs into Race Protocol has the potential to significantly enhance the verifiability, trustlessness, and security of the randomness generation process. This would further strengthen the core principles of Race Protocol and provide a more robust and trustworthy foundation for building blockchain games.

\
