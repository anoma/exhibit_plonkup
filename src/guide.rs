extern crate plonkup;

use plonkup::prelude::*;
use rand_core::OsRng;
use rand::Rng;

fn generate_xor_lookup_table_4bit(composer: &mut StandardComposer) {
    for i in 0..16 {
        for j in 0..16 {
            composer.lookup_table.0.push([
                BlsScalar::from(i as u64),
                BlsScalar::from(j as u64),
                BlsScalar::from(i^j as u64),
                BlsScalar::zero(),
            ])
        }
    }
}

fn example_circuit(composer: &mut StandardComposer, left: u8, right: u8, out: u8) {

    // split into high and low 4-bit chunks
    let left_hi = BlsScalar::from((left / 16) as u64);
    let left_lo = BlsScalar::from((left % 16) as u64);

    let right_hi = BlsScalar::from((right / 16) as u64);
    let right_lo = BlsScalar::from((right % 16) as u64);

    let out_hi = BlsScalar::from((out / 16) as u64);
    let out_lo = BlsScalar::from((out % 16) as u64);


    // prepare all inputs

    // get final output as a variable
    let out_var = composer.add_input(BlsScalar::from(out as u64));

    // turn each chunk into an input to the circuit
    let left_hi_var = composer.add_input(left_hi);
    let left_lo_var = composer.add_input(left_lo);

    let right_hi_var = composer.add_input(right_hi);
    let right_lo_var = composer.add_input(right_lo);

    let out_hi_var = composer.add_input(out_hi);
    let out_lo_var = composer.add_input(out_lo);



    // add gates to the circuit

    // lookup XOR for high and low parts
    composer.plookup_gate(
        left_hi_var,                // a
        right_hi_var,               // b
        out_hi_var,                 // c
        Some(composer.zero_var),    // Option(d)
        BlsScalar::zero(),          // pi
    );

    composer.plookup_gate(
        left_lo_var,
        right_lo_var,
        out_lo_var,
        Some(composer.zero_var),
        BlsScalar::zero(),
    );

    // pack the 4-bit chunks from out back into a byte
    composer.add_gate(
        out_hi_var,             // a
        out_lo_var,             // b
        out_var,                // c
        BlsScalar::from(16),    // q_L
        BlsScalar::from(1),     // q_R
        -BlsScalar::from(1),    // q_O
        BlsScalar::zero(),      // q_C
        BlsScalar::zero(),      // pi
    );
}

fn main() {
    let mut rng = rand::thread_rng();

    let n: usize = 512;

    eprint!("Generating parameters...");
    let public_parameters = PublicParameters::setup(n, &mut OsRng).unwrap();
    eprint!("done\n");

    // This is the public XOR table made of 4-bit chunks
    // Typically this is generated far in advance and known to both Prover and Verifier

    let (proof, public_inputs, lookup_table) = {

        // Prover wants to show they know the XOR of these private 32-bit numbers
        // this is "outside" the circuit

        let left: u8 = rng.gen();
        let right: u8 = rng.gen();
        let out = left ^ right;

        // Create a prover struct
        let mut prover = Prover::new(b"zkhack-workshop");

        // Add lookup table to the composer
        generate_xor_lookup_table_4bit(prover.mut_cs());

        // Add the 4-bit XOR circuit
        example_circuit(prover.mut_cs(), left, right, out);

        // Commit Key
        let (ck, _) = public_parameters.trim(prover.mut_cs().total_size().next_power_of_two()).unwrap();

        // Preprocess circuit
        eprint!("Prover preprocessing circuit...");
        prover.preprocess(&ck).unwrap();
        eprint!("done\n");

        // Once the prove method is called, the public inputs are cleared
        // So pre-fetch these before calling Prove
        let public_inputs = prover.mut_cs().public_inputs.clone();
        let lookup_table = prover.cs.lookup_table.clone();

        // Create proof
        eprint!("Creating proof...");
        (prover.prove(&ck).unwrap(), public_inputs, lookup_table)
    };
    eprint!("done\n");

    let mut verifier = Verifier::new(b"zkhack-workshop");

        // Add lookup table to the composer
        verifier.mut_cs().append_lookup_table(&lookup_table);

        // Add the 4-bit XOR circuit
        example_circuit(verifier.mut_cs(), 7u8, 2u8, 1u8);
  
        // Compute Commit and Verifier Key
        let (ck, vk) = public_parameters
            .trim(verifier.mut_cs().total_size().next_power_of_two()).unwrap();
    
        // Preprocess circuit
        eprint!("Verifier preprocessing...");
        verifier.preprocess(&ck).unwrap();
        eprint!("done\n");
    
        // Verify proof
        eprint!("Verifying...");
        let result = verifier.verify(&proof, &vk, &public_inputs, &lookup_table);
        eprint!("done\n");

        if result.is_ok() { eprint!("Proof accepted!\n")} else {eprint!("Proof rejected\n")};
}

