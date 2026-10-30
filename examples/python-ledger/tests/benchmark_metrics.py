"""Finite completion fairness for the native API's 128-request windows."""


def pending_completion_gaps(order, groups):
    """Count intervening completions before service while offered work remains.

    A finished group receives no penalty for other groups completing later.
    Counts describe finite coverage, not starvation freedom.
    """
    if groups < 1 or any(group < 0 or group >= groups for group in order):
        raise ValueError("invalid completion group")
    previous = [-1] * groups
    gaps = [0] * groups
    counts = [0] * groups
    for position, group in enumerate(order):
        gaps[group] = max(gaps[group], position - previous[group] - 1)
        previous[group] = position
        counts[group] += 1
    return dict(counts=counts, max_other_completions_while_pending=gaps)


def assigned_owner_fairness(ready_ns, owners, *, first_owner=0):
    """Attribute by the verified round-robin API contract, within each window.

    Native timestamps restart at admission of each 128-request window. They
    cannot be sorted across windows or treated as whole-call response times.
    Equal timestamps are ordered by input index for reproducible diagnostics.
    """
    if owners < 1 or not 0 <= first_owner < owners or any(value < 0 for value in ready_ns):
        raise ValueError("invalid native timing or owner count")
    counts = [0] * owners
    overtakes = [0] * owners
    gaps_ns = [0] * owners
    for offset in range(0, len(ready_ns), 128):
        window = ready_ns[offset:offset + 128]
        ordered = sorted(enumerate(window), key=lambda pair: (pair[1], pair[0]))
        order = [(first_owner + offset + index) % owners for index, _ in ordered]
        gaps = pending_completion_gaps(order, owners)
        previous_ns = [0] * owners
        for index, value in ordered:
            owner = (first_owner + offset + index) % owners
            gaps_ns[owner] = max(gaps_ns[owner], value - previous_ns[owner])
            previous_ns[owner] = value
        for owner in range(owners):
            counts[owner] += gaps["counts"][owner]
            overtakes[owner] = max(overtakes[owner], gaps["max_other_completions_while_pending"][owner])
    return dict(assigned_owner_counts=counts,
                assigned_owner_max_other_completions_while_pending=overtakes,
                assigned_owner_max_pending_gap_ns=gaps_ns)
