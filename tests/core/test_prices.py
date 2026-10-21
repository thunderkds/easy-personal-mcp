import pytest

from easy_personal.core import prices


def test_packaged_table_is_dated():
    table = prices.load()
    assert table.as_of  # ISO date string
    assert table.models


def test_cost_is_tokens_times_price_per_million(tmp_path):
    path = tmp_path / "p.yaml"
    path.write_text(
        "as_of: 2026-01-01\nmodels:\n  m-known: input=3 output=15 cache_write=3.75 cache_read=0.3\n"
    )
    table = prices.load(path)
    assert table.as_of == "2026-01-01"
    usd = table.cost("m-known", 1_000_000, 1_000_000, 1_000_000, 1_000_000)
    assert usd == pytest.approx(3 + 15 + 3.75 + 0.3)


def test_unknown_or_missing_model_abstains(tmp_path):
    path = tmp_path / "p.yaml"
    path.write_text(
        "as_of: 2026-01-01\nmodels:\n  m: input=1 output=1 cache_write=1 cache_read=1\n"
    )
    table = prices.load(path)
    assert table.cost("other", 1, 1, 1, 1) is None
    assert table.cost(None, 1, 1, 1, 1) is None
