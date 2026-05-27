namespace QuantumSamples.Grover {
    open Microsoft.Quantum.Canon;
    open Microsoft.Quantum.Intrinsic;
    open Microsoft.Quantum.Math;
    open Microsoft.Quantum.Convert;
    open Microsoft.Quantum.Measurement;
    open Microsoft.Quantum.Arrays;

    // NOTE: optimal Grover iteration count is floor(pi/4 * sqrt(N/M)) where
    // N = 2^nQubits and M = number of marked states in the search space
    function IterationCount(nQubits : Int) : Int {
        // TODO: accept M as a parameter -- this assumes exactly one solution exists
        let n = IntAsDouble(nQubits);
        return Floor(PI() / 4.0 * Sqrt(PowD(2.0, n)));
    }

    operation ReflectAboutUniform(qubits : Qubit[]) : Unit is Adj + Ctl {
        within {
            ApplyToEachA(H, qubits);
            ApplyToEachA(X, qubits);
        } apply {
            Controlled Z(Most(qubits), Tail(qubits));
        }
    }

    operation GroverSearch(
        nQubits : Int,
        oracle : (Qubit[] => Unit is Adj)
    ) : Result[] {
        // FIXME: returns garbage when nQubits < 2 -- minimum register size is not validated
        use qubits = Qubit[nQubits];
        ApplyToEach(H, qubits);
        let iters = IterationCount(nQubits);
        for _ in 1..iters {
            oracle(qubits);
            ReflectAboutUniform(qubits);
        }
        return MeasureEachZ(qubits);
    }

    // HACK: oracle hard-codes the target state |1010> instead of accepting a
    // classical bitstring -- generalise once the adjoint generation is verified
    operation MarkTarget(qubits : Qubit[]) : Unit is Adj {
        within {
            X(qubits[1]);
            X(qubits[3]);
        } apply {
            Controlled Z(Most(qubits), Tail(qubits));
        }
    }

    @EntryPoint()
    operation RunGrover() : Result[] {
        // XXX: no fallback if the backend does not support the full Clifford+T gate set
        return GroverSearch(4, MarkTarget);
    }
}
