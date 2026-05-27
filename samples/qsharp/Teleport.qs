namespace QuantumSamples.Teleport {
    open Microsoft.Quantum.Intrinsic;
    open Microsoft.Quantum.Canon;
    open Microsoft.Quantum.Measurement;

    // NOTE: quantum teleportation transfers state, not matter -- the source qubit
    // is destroyed; this does not violate no-cloning because the original is gone
    operation CreateBellPair(alice : Qubit, bob : Qubit) : Unit {
        H(alice);
        CNOT(alice, bob);
    }

    operation MeasureAndSend(msg : Qubit, alice : Qubit) : (Result, Result) {
        CNOT(msg, alice);
        H(msg);
        // TODO: replace tuple return with a proper classical channel abstraction
        // so simulated and hardware backends can be swapped without changing callers
        return (M(msg), M(alice));
    }

    operation ApplyCorrections(bob : Qubit, aliceBit : Result, msgBit : Result) : Unit {
        // OPTIMIZE: on hardware that exposes parallelism, both gates can be applied
        // simultaneously since X and Z commute on separate qubits
        if aliceBit == One { X(bob); }
        if msgBit == One   { Z(bob); }
    }

    operation Teleport(msg : Qubit, bob : Qubit) : Unit {
        use alice = Qubit();
        CreateBellPair(alice, bob);
        let (aliceBit, msgBit) = MeasureAndSend(msg, alice);
        ApplyCorrections(bob, aliceBit, msgBit);
        // FIXME: alice is released at end of scope without an explicit Reset --
        // this raises ReleasedQubitsAreNotInZeroState on the full-state simulator
        Reset(alice);
    }

    // DEPRECATED: use Teleport() -- this shim exists only for v0.x call-site compatibility
    operation TeleportLegacy(source : Qubit, target : Qubit) : Unit {
        Teleport(source, target);
    }

    @EntryPoint()
    operation RunTeleport() : Result {
        use (msg, bob) = (Qubit(), Qubit());
        H(msg);
        Teleport(msg, bob);
        let result = M(bob);
        Reset(bob);
        return result;
    }
}
