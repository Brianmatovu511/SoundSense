const int soundPin = A0;

const int greenLED = 4;
const int yellowLED = 5;
const int redLED = 6;

void setup() {
  pinMode(greenLED, OUTPUT);
  pinMode(yellowLED, OUTPUT);
  pinMode(redLED, OUTPUT);

  Serial.begin(9600);
}

void loop() {
  int soundValue = analogRead(soundPin);

  // Turn all LEDs off first
  digitalWrite(greenLED, LOW);
  digitalWrite(yellowLED, LOW);
  digitalWrite(redLED, LOW);

  if (soundValue < 187) {
    digitalWrite(greenLED, HIGH);
  } 
  else if (soundValue < 190) {
    digitalWrite(yellowLED, HIGH);
  } 
  else {
    digitalWrite(redLED, HIGH);
  }

  // Send data to computer (for backend later)
  Serial.print("SOUND:");
  Serial.println(soundValue);

  delay(200);
}
