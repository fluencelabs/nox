package fluence.merkle

import java.nio.ByteBuffer

import fluence.merkle

import scala.collection.mutable

class ByteBufferWrapper(bb: ByteBuffer, chunkSize: Int) {

  private val touchedChunks = mutable.Set.empty[Int]

  def getTouchedAndReset(): Set[Int] = {
    val result = touchedChunks.toSet
    touchedChunks.clear()
    result
  }

  private def touch(i: Int) = {
    touchedChunks.add(i / chunkSize)
  }

  def slice(): ByteBuffer = bb.slice()

  def duplicate(): ByteBufferWrapper = {
    new ByteBufferWrapper(bb.slice(), chunkSize)
  }

  def asReadOnlyBuffer(): ByteBufferWrapper = new ByteBufferWrapper(bb.slice(), chunkSize)

  def get(): Byte = bb.get()

  def get(dst: Array[Byte], offset: Int, length: Int): Unit = {
    bb.get(dst, offset, length)
  }

  def put(b: Byte) = {
    val index = bb.position() + 1
    bb.put(b)
    touch(index)
    this
  }

  def get(index: Int) = bb.get(index)

  def put(index: Int, b: Byte) = {
    bb.put(index, b)
    touch(index)
    this
  }

  def compact() = {
    bb.compact()
    this
  }

  def isDirect = bb.isDirect

  def getChar = bb.getChar()

  def putChar(value: Char) = {
    bb.putChar(value)
    this
  }

  def getChar(index: Int) = bb.getChar(index)

  def putChar(index: Int, value: Char) = {
    bb.putChar(index, value)
    this
  }

  def asCharBuffer() = throw new RuntimeException("Еhe method cannot be used")

  def getShort = bb.getShort()

  def putShort(value: Short) = {
    bb.putShort(value)
    this
  }

  def getShort(index: Int) = bb.getShort(index)

  def putShort(index: Int, value: Short) = {
    bb.putShort(index, value)
    this
  }

  def asShortBuffer() = throw new RuntimeException("Еhe method cannot be used")

  def getInt = bb.getInt()

  def putInt(value: Int) = {
    bb.putInt(value)
    this
  }

  def getInt(index: Int) = ???

  def putInt(index: Int, value: Int) = ???

  def asIntBuffer() = ???

  def getLong = ???

  def putLong(value: Long) = ???

  def getLong(index: Int) = ???

  def putLong(index: Int, value: Long) = ???

  def asLongBuffer() = ???

  def getFloat = ???

  def putFloat(value: Float) = ???

  def getFloat(index: Int) = ???

  def putFloat(index: Int, value: Float) = ???

  def asFloatBuffer() = ???

  def getDouble = ???

  def putDouble(value: Double) = ???

  def getDouble(index: Int) = ???

  def putDouble(index: Int, value: Double) = ???

  def asDoubleBuffer() = ???

  def isReadOnly = bb.isReadOnly

  def capacity = bb.capacity

  def clear() = {
    bb.clear
    this
  }

  def flip = {
    bb.flip
    this
  }

  def hasRemaining = bb.hasRemaining

  def limit = bb.limit

  def limit(newLimit: Int) = {
    bb.limit(newLimit)
    this
  }

  def mark = {
    bb.mark
    this
  }

  def position = bb.position

  def position(newPosition: Int) = {
    bb.position(newPosition)
    this
  }

  def remaining() = bb.remaining

  def reset = {
    bb.reset
    this
  }

  def rewind = {
    bb.rewind
    this
  }

  def array = bb.array

  def arrayOffset = bb.arrayOffset
}

object ByteBufferWrapper {

  def allocate(capacity: Int) = {
    new merkle.ByteBufferWrapper(ByteBuffer.allocate(capacity), 100)
  }

  def allocateDirect(capacity: Int) = {
    new merkle.ByteBufferWrapper(ByteBuffer.allocateDirect(capacity), 100)
  }
}
