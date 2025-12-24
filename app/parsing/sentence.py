"""分句 - Phase M1 实现"""

import re
from typing import List


def split_sentences(text: str) -> List[str]:
    """
    中文分句（简单实现）

    Args:
        text: 原始文本

    Returns:
        句子列表
    """
    # 简单的分句逻辑：按句号、问号、感叹号分割
    sentences = re.split(r'[。！？]', text)
    # 过滤空字符串并去除空白
    return [s.strip() for s in sentences if s.strip()]


def split_words(sentence: str) -> List[str]:
    """
    分词（占位实现）

    Args:
        sentence: 句子

    Returns:
        单词列表
    """
    # 简单实现：按空格分割
    return sentence.split()