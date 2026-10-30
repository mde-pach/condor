import abc
from abc import ABC, abstractmethod


class Shape(ABC):
    @abstractmethod
    def area(self, precision):
        pass

    @abc.abstractmethod  # dead: draw
    def draw(self, canvas):
        pass

    def helper(self, value):  # dead: helper, value
        pass


class Square(Shape):
    def area(self, precision):
        return precision


print(Shape, Square().area)
