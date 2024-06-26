**Input:** A qubit in state $|\psi\rangle = \beta |0\rangle + \gamma |1\rangle$.

**Goal**: Change the state of the qubit to $- \beta |0\rangle - \gamma |1\rangle$.

> This change on its own is not observable - there is no experiment you can do on a standalone qubit to figure out whether it acquired the global phase or not.
> However, you can use a controlled version of this operation to observe the global phase it introduces.
> This is used in later katas as part of more complicated tasks.
