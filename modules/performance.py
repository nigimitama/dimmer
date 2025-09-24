import time
import functools
from contextlib import contextmanager
from typing import Callable, Any, Iterator, Optional


@contextmanager
def timer(name: str = "Operation", verbose: bool = True) -> Iterator[dict]:
    """
    実行時間を測定するコンテクストマネージャー

    Args:
        name: 測定対象の名前
        verbose: 結果を出力するかどうか

    Yields:
        dict: 実行時間の情報を含む辞書
    """
    start_time = time.perf_counter()
    result = {"name": name, "start_time": start_time, "end_time": None, "duration": None}

    try:
        yield result
    finally:
        end_time = time.perf_counter()
        duration = end_time - start_time

        result["end_time"] = end_time
        result["duration"] = duration

        if verbose:
            print(f"⏱️  {name}: {duration:.4f}秒")


def measure_time(name: Optional[str] = None, verbose: bool = True):
    """
    関数の実行時間を測定するデコレータ

    Args:
        name: 測定対象の名前（Noneの場合は関数名を使用）
        verbose: 結果を出力するかどうか
    """

    def decorator(func: Callable) -> Callable:
        @functools.wraps(func)
        def wrapper(*args, **kwargs) -> Any:
            operation_name = name or f"{func.__module__}.{func.__name__}"

            with timer(operation_name, verbose) as timing_info:
                result = func(*args, **kwargs)
                # timing_infoに関数の結果も保存（必要に応じて）
                timing_info["result"] = result
                return result

        return wrapper

    return decorator
