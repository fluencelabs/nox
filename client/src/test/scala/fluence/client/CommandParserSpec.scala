package fluence.client

import org.scalatest.{ Matchers, WordSpec }

class CommandParserSpec extends WordSpec with Matchers {

  //todo test with escape characters
  "command parser" should {
    "parse all commands correctly" in {
      CommandParser.parseCommand("exit and some '\" other string").get shouldBe Exit
      CommandParser.parseCommand("exit").get shouldBe Exit

      val key = "somekey"
      val value = "somevalue"
      CommandParser.parseCommand(s"""put $key $value""").get shouldBe Put(key, value)
      CommandParser.parseCommand(s"""put '$key' '$value' """).get shouldBe Put(key, value)
      CommandParser.parseCommand(s"""put '$key' $value """).get shouldBe Put(key, value)
      CommandParser.parseCommand(s"""put '$key' "$value" """).get shouldBe Put(key, value)
      CommandParser.parseCommand(s"""put $key "$value" """).get shouldBe Put(key, value)
      CommandParser.parseCommand(s"""put "$key" "$value" """).get shouldBe Put(key, value)
      CommandParser.parseCommand(s"""put "$key" $value""").get shouldBe Put(key, value)
      CommandParser.parseCommand(s"""put "$key" '$value'""").get shouldBe Put(key, value)

      CommandParser.parseCommand(s"""get $key """).get shouldBe Get(key)
      CommandParser.parseCommand(s"""get '$key' """).get shouldBe Get(key)
      CommandParser.parseCommand(s"""get '$key'  """).get shouldBe Get(key)
      CommandParser.parseCommand(s"""get '$key'  """).get shouldBe Get(key)
      CommandParser.parseCommand(s"""get $key  """).get shouldBe Get(key)
      CommandParser.parseCommand(s"""get "$key"  """).get shouldBe Get(key)
      CommandParser.parseCommand(s"""get "$key" """).get shouldBe Get(key)
      CommandParser.parseCommand(s"""get "$key" """).get shouldBe Get(key)
    }
  }

}
