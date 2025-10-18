#!/usr/bin/env python3

import timeit
import humanize
import humanize._fast
from tqdm import tqdm

# Sample data
nums = list(range(1, 1_000_001))

# Benchmark Python version
python_time = timeit.timeit(
    stmt="for n in tqdm(nums): humanize.intcomma(n)",
    globals=globals(),
    number=1
)

# Benchmark Rust version
rust_time = timeit.timeit(
    stmt="for n in tqdm(nums): humanize._fast.intcomma(n)",
    globals=globals(),
    number=1
)

print(f"Python time: {python_time:.4f} s")
print(f"Rust time:   {rust_time:.4f} s")
print(f"Speedup:     {python_time / rust_time:.2f}x")
