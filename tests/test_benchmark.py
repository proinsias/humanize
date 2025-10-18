import humanize.number
import humanize._fast

nums = list(range(1, 1_000_001))


def test_python_benchmark(benchmark):
    benchmark(lambda: [humanize.number.intcomma(n) for n in nums])


def test_rust_benchmark(benchmark):
    benchmark(lambda: [humanize._fast.intcomma(n) for n in nums])


def test_python_iterable_benchmark(benchmark):
    benchmark(lambda: humanize.number.intcomma([n for n in nums]))


def test_rust_iterable_benchmark(benchmark):
    benchmark(lambda: humanize._fast.intcomma([n for n in nums]))
