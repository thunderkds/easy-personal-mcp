"""Dated API price table -> API-equivalent cost. A model not in the table abstains (None)."""

from dataclasses import dataclass
from pathlib import Path

PACKAGED = Path(__file__).resolve().parent.parent / "data" / "prices.yaml"
PER = 1_000_000


@dataclass(frozen=True)
class Price:
    """USD per million tokens."""

    input: float
    output: float
    cache_write: float
    cache_read: float


@dataclass(frozen=True)
class PriceTable:
    as_of: str
    models: dict[str, Price]

    def cost(self, model, input_tokens, output_tokens, cache_creation, cache_read) -> float | None:
        price = self.models.get(model) if model else None
        if price is None:
            return None
        return (
            input_tokens * price.input
            + output_tokens * price.output
            + cache_creation * price.cache_write
            + cache_read * price.cache_read
        ) / PER


def load(path: Path | None = None) -> PriceTable:
    """Read the flat `as_of:` + `models:` file (one `name: key=value ...` line per model)."""
    as_of, models, in_models = "", {}, False
    for line in (path or PACKAGED).read_text().splitlines():
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        if not line.startswith(" "):
            key, _, value = line.partition(":")
            in_models = key == "models"
            if key == "as_of":
                as_of = value.strip()
        elif in_models:
            name, _, rest = line.strip().partition(":")
            fields = dict(item.split("=") for item in rest.split())
            models[name] = Price(**{k: float(v) for k, v in fields.items()})
    return PriceTable(as_of=as_of, models=models)
