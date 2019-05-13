package fluence.merkle

import java.nio.{ByteBuffer, ByteOrder}
import java.util

import asmble.compile.jvm.MemoryBuffer

class TrackingMemoryBuffer(val bb: ByteBuffer, size: Int, chunkSize: Int) extends MemoryBuffer {
  import TrackingMemoryBuffer._

  private val dirtyChunks = new util.BitSet(size / chunkSize)

  def getElements(offset: Int, length: Int): Array[Byte] = {
    val arr = new Array[Byte](length)
    val newBb = bb.duplicate()
    bb.position(offset)
    bb.get(arr, 0, length)
    arr
  }

  def getDirtyChunks: util.BitSet = dirtyChunks

  def slice(): ByteBuffer = bb.slice()

  def duplicate(): MemoryBuffer = {
    new TrackingMemoryBuffer(bb.duplicate(), size, chunkSize)
  }

  def get(): Byte = bb.get()

  def get(dst: Array[Byte], offset: Int, length: Int): Unit = {
    bb.get(dst, offset, length)
  }

  def put(b: Byte): TrackingMemoryBuffer = {
    val index = bb.position()
    bb.put(b)
    dirtyChunks.set(index / chunkSize)
    this
  }

  def get(index: Int): Byte = bb.get(index)

  def put(index: Int, b: Byte): TrackingMemoryBuffer = {
    bb.put(index, b)
    getDirtyChunks.set(index / chunkSize)
    this
  }

  def getShort(index: Int): Short = bb.getShort(index)

  def putShort(index: Int, value: Short): MemoryBuffer = {
    bb.putShort(index, value)
    val from = index / chunkSize
    val to = (index + SHORT_SIZE) / chunkSize
    if (from == to) getDirtyChunks.set(from)
    else getDirtyChunks.set(from, to)
    this
  }

  def capacity: Int = bb.capacity

  def clear(): MemoryBuffer = {
    bb.clear
    this
  }

  def limit: Int = bb.limit

  def limit(newLimit: Int): MemoryBuffer = {
    bb.limit(newLimit)
    this
  }

  def position: Int = bb.position

  def position(newPosition: Int): MemoryBuffer = {
    bb.position(newPosition)
    this
  }

  override def order(order: ByteOrder): MemoryBuffer = {
    bb.order(order)
    this
  }

  override def put(arr: Array[Byte], offset: Int, length: Int): MemoryBuffer = {
    bb.put(arr, offset, length)
    getDirtyChunks.set(offset / chunkSize, (offset + length) / chunkSize)
    this
  }

  override def put(arr: Array[Byte]): MemoryBuffer = {
    val pos = bb.position()
    bb.put(arr)
    getDirtyChunks.set(pos / chunkSize, (pos + arr.length) / chunkSize)
    this
  }

  override def get(arr: Array[Byte]): MemoryBuffer = {
    bb.get(arr)
    this
  }

  override def putInt(index: Int, n: Int): MemoryBuffer = {
    bb.putInt(index, n)
    val from = index / chunkSize
    val to = (index + INT_SIZE) / chunkSize
    if (from == to) getDirtyChunks.set(from)
    else getDirtyChunks.set(from, to)
    this
  }

  override def putLong(index: Int, n: Long): MemoryBuffer = {
    bb.putLong(index, n)
    val from = index / chunkSize
    val to = (index + LONG_SIZE) / chunkSize
    if (from == to) getDirtyChunks.set(from)
    else getDirtyChunks.set(from, to)
    this
  }

  override def putDouble(index: Int, n: Double): MemoryBuffer = {
    bb.putDouble(index, n)
    val from = index / chunkSize
    val to = (index + DOUBLE_SIZE) / chunkSize
    if (from == to) getDirtyChunks.set(from)
    else getDirtyChunks.set(from, to)
    this
  }

  override def putFloat(index: Int, n: Float): MemoryBuffer = {
    bb.putFloat(index, n)
    val from = index / chunkSize
    val to = (index + FLOAT_SIZE) / chunkSize
    if (from == to) getDirtyChunks.set(from)
    else getDirtyChunks.set(from, to)
    this
  }

  override def getInt(index: Int): Int = bb.getInt(index)

  override def getLong(index: Int): Long = bb.getLong(index)

  override def getFloat(index: Int): Float = bb.getFloat(index)

  override def getDouble(index: Int): Double = bb.getDouble(index)
}

object TrackingMemoryBuffer {

  val SHORT_SIZE = 2
  val INT_SIZE = 4
  val LONG_SIZE = 8
  val FLOAT_SIZE = 4
  val DOUBLE_SIZE = 8

  def allocate(capacity: Int, chunkSize: Int): TrackingMemoryBuffer = {
    new TrackingMemoryBuffer(ByteBuffer.allocate(capacity), capacity, chunkSize)
  }

  def allocateDirect(capacity: Int, chunkSize: Int): TrackingMemoryBuffer = {
    new TrackingMemoryBuffer(ByteBuffer.allocateDirect(capacity), capacity, chunkSize)
  }
}
