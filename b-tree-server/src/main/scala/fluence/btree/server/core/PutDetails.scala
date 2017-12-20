package fluence.btree.server.core

import fluence.btree.client.{ Key, Value }

import scala.collection.Searching.SearchResult

/**
 * Structure for holding all details needed for putting key and value to BTree.
 *
 * @param key          The key that will be placed to the BTree
 * @param value        The value that will be placed to the BTree
 * @param searchResult A result of searching client key in server leaf keys. Contains an index
 *                     for putting specified key and value
 */
case class PutDetails(key: Key, value: Value, searchResult: SearchResult)
